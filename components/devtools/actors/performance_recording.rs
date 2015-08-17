/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use actor::{Actor, ActorRegistry, ActorMessageStatus};
use actors::timeline::HighResolutionStamp;
use protocol::JsonPacketStream;

use rustc_serialize::{Encoder, Encodable};
use rustc_serialize::json::{self, Json};
use time;

use std::cell::Cell;
use std::collections::BTreeMap;
use std::net::TcpStream;

pub struct PerformanceRecordingActor {
    name: String,

    configuration: Configuration,
    start_time: u64,
    completed: Cell<bool>,
}

#[derive(Copy, Clone, Default, RustcEncodable)]
pub struct Configuration {
    pub withMarkers: bool,
    pub withTicks: bool,
    pub withMemory: bool,
    pub withAllocations: bool,
    pub withJITOptimizations: bool,
    pub allocationsSampleProbability: f64,
    pub allocationsMaxLogLength: u64,
    pub bufferSize: u64,
    pub sampleFrequency: u64,
}

impl Configuration {
    pub fn new(options: Option<&BTreeMap<String, Json>>) -> Configuration {
        let mut configuration = Configuration::default();
        if let Some(options) = options {
            if let Some(&Json::Boolean(withMarkers)) = options.get("withMarkers") {
                configuration.withMarkers = withMarkers;
            }
            if let Some(&Json::Boolean(withTicks)) = options.get("withTicks") {
                configuration.withTicks = withTicks;
            }
            if let Some(&Json::Boolean(withMemory)) = options.get("withMemory") {
                configuration.withMemory = withMemory;
            }
            if let Some(&Json::Boolean(withJITOptimizations)) = options.get("withJITOptimizations") {
                configuration.withJITOptimizations = withJITOptimizations;
            }
            if let Some(&Json::Boolean(withAllocations)) = options.get("withAllocations") {
                configuration.withAllocations = withAllocations;
            }
            if let Some(&Json::F64(allocationsSampleProbability)) = options.get("allocationsSampleProbability") {
                configuration.allocationsSampleProbability = allocationsSampleProbability;
            }
            if let Some(&Json::U64(allocationsMaxLogLength)) = options.get("allocationsMaxLogLength") {
                configuration.allocationsMaxLogLength = allocationsMaxLogLength;
            }
            if let Some(&Json::U64(bufferSize)) = options.get("bufferSize") {
                configuration.bufferSize = bufferSize;
            }
            if let Some(&Json::U64(sampleFrequency)) = options.get("sampleFrequency") {
                configuration.sampleFrequency = sampleFrequency;
            }
        }
        configuration
    }
}

#[derive(Clone, RustcEncodable)]
pub struct StartingBufferStatus {
    position: u32,
    totalSize: u32,
    generation: u32,
}

#[derive(Clone, RustcEncodable)]
pub struct PerformanceRecording {
    actor: String,
    configuration: Configuration,
    startingBufferStatus: StartingBufferStatus,
    console: bool,
    label: String,
    startTime: f64,
    localStartTime: u64,
    recording: bool,
    completed: bool,
    duration: u64,
    //profile: Option<...>,
}

#[derive(Clone, RustcEncodable)]
pub struct PerformanceRecordingWithActor {
    actor: String,
    configuration: Configuration,
    startingBufferStatus: StartingBufferStatus,
    console: bool,
    label: String,
    startTime: f64,
    localStartTime: u64,
    recording: bool,
    completed: bool,
    duration: u64,
    from: String,
    //profile: Option<...>,
}

/*
#[derive(RustcEncodable)]
struct TimelineMarkerReply {
    name: String,
    start: HighResolutionStamp,
    end: HighResolutionStamp,
    stack: Option<Vec<()>>,
    endStack: Option<Vec<()>>,
}

#[derive(RustcEncodable)]
struct MarkersEmitterData {
    markers: Vec<TimelineMarkerReply>,
    endTime: HighResolutionStamp,
}

#[derive(RustcEncodable)]
struct MemoryEmitterReply {
    __type__: String,
    from: String,
    name: String,
    data: MarkersEmitterData,
    recordings: Vec<PerformanceRecording>,
}
*/


impl Actor for PerformanceRecordingActor {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn handle_message(&self,
                      _registry: &ActorRegistry,
                      _msg_type: &str,
                      _msg: &json::Object,
                      _stream: &mut TcpStream) -> Result<ActorMessageStatus, ()> {
        Ok(ActorMessageStatus::Ignored)
    }
}

/// Returns the number of milliseconds elapsed since the Unix epoch
/// (1 January 1970 00:00:00 UTC). Similar to JS `Date.now()`.
fn now() -> u64 {
    let spec = time::now().to_timespec();
    (spec.sec as u64) * 1000 + (spec.nsec as u64) / 1000000
}

impl PerformanceRecordingActor {
    pub fn new(name: String, configuration: Configuration) -> PerformanceRecordingActor {
        assert_eq!(configuration.withMarkers, true);
        assert_eq!(configuration.withTicks, true);
        assert_eq!(configuration.withMemory, false);
        assert_eq!(configuration.withAllocations, false);
        assert_eq!(configuration.withJITOptimizations, false);

        PerformanceRecordingActor {
            name: name,

            configuration: configuration,
            start_time: now(),
            completed: Cell::new(false),
        }
    }

    pub fn set_completed(&self) {
        self.completed.set(true);
    }

    pub fn encodable(&self) -> PerformanceRecording {
        PerformanceRecording {
            actor: self.name(),
            configuration: self.configuration,
            startingBufferStatus: StartingBufferStatus {
                position: 131004,
                totalSize: 10000000,
                generation: 0,
            },
            console: false,
            label: "".to_owned(),
            startTime: 6404439.779669,
            localStartTime: self.start_time,
            recording: true,
            completed: self.completed.get(),
            duration: 0,
        }
    }

    pub fn encodable_with_actor(&self, actor: String) -> PerformanceRecordingWithActor {
        PerformanceRecordingWithActor {
            actor: self.name(),
            configuration: self.configuration,
            startingBufferStatus: StartingBufferStatus {
                position: 131004,
                totalSize: 10000000,
                generation: 0,
            },
            console: false,
            label: "".to_owned(),
            startTime: 6404439.779669,
            localStartTime: self.start_time,
            recording: true,
            completed: self.completed.get(),
            duration: 0,
            from: actor,
        }
    }
}

impl Encodable for PerformanceRecordingActor {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        let encodable = self.encodable();
        encodable.encode(s)
    }
}
