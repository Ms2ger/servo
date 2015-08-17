/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use actor::{Actor, ActorMessageStatus, ActorRegistry};
use actors::framerate::FramerateActor;
use actors::performance_recording::{PerformanceRecordingActor, PerformanceRecording, Configuration};
use actors::timeline::Emitter;
use protocol::JsonPacketStream;

use devtools_traits::{DevtoolScriptControlMsg, TimelineMarker, TimelineMarkerType};
use msg::constellation_msg::PipelineId;
use util::task::spawn_named;

use ipc_channel::ipc::{self, IpcSender, IpcReceiver};
use rustc_serialize::json::{self, Json};

use std::cell::RefCell;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep_ms;

static DEFAULT_TIMELINE_DATA_PULL_TIMEOUT: u32 = 200; //ms

pub struct PerformanceActor {
    name: String,
    pipeline: PipelineId,
    script_sender: IpcSender<DevtoolScriptControlMsg>,
    recordings: RefCell<Vec<String>>,
    is_recording: Arc<AtomicBool>,
    stream: RefCell<Option<TcpStream>>,
}

#[derive(RustcEncodable)]
struct PerformanceFeatures {
    withMarkers: bool,
    withMemory: bool,
    withTicks: bool,
    withAllocations: bool,
    withJITOptimizations: bool,
}

#[derive(RustcEncodable)]
struct PerformanceTraits {
    features: PerformanceFeatures,
}

#[derive(RustcEncodable)]
struct ConnectReply {
    from: String,
    traits: PerformanceTraits,
}

#[derive(RustcEncodable)]
struct StartStoppingRecordingReply {
    from: String,
    __type__: String,
    recording: PerformanceRecording,
}

#[derive(RustcEncodable)]
struct StopRecordingReply {
    from: String,
    __type__: String,
    recording: PerformanceRecording,
    data: json::Object,
}

impl Actor for PerformanceActor {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn handle_message(&self,
                      registry: &ActorRegistry,
                      msg_type: &str,
                      _msg: &json::Object,
                      stream: &mut TcpStream) -> Result<ActorMessageStatus, ()> {
        Ok(match msg_type {
            "connect" => {
                let msg = ConnectReply {
                    from: self.name(),
                    traits: PerformanceTraits {
                        features: PerformanceFeatures {
                            withMarkers: true,
                            withMemory: false,
                            withTicks: true,
                            withAllocations: false,
                            withJITOptimizations: false,
                        },
                    },
                };
                stream.write_json_packet(&msg);
                ActorMessageStatus::Processed
            },
            "startRecording" => {
                println!("startRecording({:?})", _msg);
                let configuration = Configuration::new(_msg.get("options").and_then(|o| o.as_object()));
                let framerate_actor = if configuration.withTicks {
                    Some(FramerateActor::create(
                        registry,
                        self.pipeline.clone(),
                        self.script_sender.clone()))
                } else {
                    None
                };


                let recording = PerformanceRecordingActor::new(
                    registry.new_name("performance-recording"),
                    configuration);
                let msg = StartStoppingRecordingReply {
                    from: self.name(),
                    __type__: "recording-started".to_owned(),
                    recording: recording.encodable(),
                };
                stream.write_json_packet(&msg);
                stream.write_json_packet(&recording.encodable_with_actor(self.name()));
                self.recordings.borrow_mut().push(recording.name());

                // init framerate actor


                let actors = registry.shareable();
                let mut emitter = Emitter::new(self.name(), actors.clone(),
                                           registry.start_stamp(),
                                           stream.try_clone().unwrap(),
                                           None,
                                           framerate_actor);
                emitter.add_recording(&recording);

                registry.register_later(box recording);


                let (sender, receiver) = ipc::channel::<TimelineMarker>().unwrap();
                let msg = DevtoolScriptControlMsg::SetTimelineMarkers(
                    self.pipeline,
                    vec![TimelineMarkerType::Reflow, TimelineMarkerType::DOMEvent],
                    sender);
                self.script_sender.send(msg).unwrap();

                *self.stream.borrow_mut() = stream.try_clone().ok();

                self.is_recording.store(true, Ordering::SeqCst);
                self.pull_timeline_data(receiver, emitter, self.is_recording.clone());

                ActorMessageStatus::Processed
            },
            "stopRecording" => {
                self.stop_recording(registry, _msg, stream);
                ActorMessageStatus::Processed
            },
            _ => ActorMessageStatus::Ignored,
        })
    }
}

impl PerformanceActor {
    pub fn new(name: String,
               pipeline: PipelineId,
               script_sender: IpcSender<DevtoolScriptControlMsg>)
               -> PerformanceActor {
        PerformanceActor {
            name: name,
            pipeline: pipeline,
            script_sender: script_sender,
            recordings: RefCell::new(Vec::new()),
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: RefCell::new(None),
        }
    }

    fn pull_timeline_data(&self, receiver: IpcReceiver<TimelineMarker>,
                          mut emitter: Emitter,
                          is_recording: Arc<AtomicBool>) {
        spawn_named("PullTimelineMarkers".to_owned(), move || {
            while is_recording.load(Ordering::SeqCst) {
                let mut markers = vec![];
                while let Ok(marker) = receiver.try_recv() {
                    markers.push(emitter.marker(marker));
                }
                emitter.send(markers);

                sleep_ms(DEFAULT_TIMELINE_DATA_PULL_TIMEOUT);
            }
        });
    }

    fn stop_recording(&self, registry: &ActorRegistry,
                      msg: &json::Object,
                      stream: &mut TcpStream) {
          println!("stopRecording({:?})", msg);
          let actor = match msg.get("options") {
              Some(&Json::String(ref actor)) => actor,
              _ => return,
          };
          let position = match self.recordings.borrow().iter().position(|a| a == actor) {
              Some(position) => position,
              None => return,
          };

          let actor = registry.find::<PerformanceRecordingActor>(actor);

          let reply = StartStoppingRecordingReply {
              from: self.name(),
              __type__: "recording-stopping".to_owned(),
              recording: actor.encodable(),
          };
          stream.write_json_packet(&reply);

          self.recordings.borrow_mut().swap_remove(position);

          self.is_recording.store(self.recordings.borrow().len() != 0, Ordering::SeqCst);
          actor.set_completed();

          let reply = StopRecordingReply {
              from: self.name(),
              __type__: "recording-stopped".to_owned(),
              recording: actor.encodable(),
              data: json::Object::new(),
          };
          stream.write_json_packet(&reply);
    }
}
