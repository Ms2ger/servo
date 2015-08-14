/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use actor::{Actor, ActorRegistry, ActorMessageStatus};

use rustc_serialize::json::{self, Json};
use std::cell::RefCell;
use std::net::TcpStream;

pub struct ProfilerActor {
    name: String,
    events: RefCell<Vec<String>>
}

impl Actor for ProfilerActor {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn handle_message(&self,
                      _registry: &ActorRegistry,
                      msg_type: &str,
                      msg: &json::Object,
                      _stream: &mut TcpStream) -> Result<ActorMessageStatus, ()> {
        Ok(match msg_type {
            "registerEventNotifications" => {
                if let Some(&Json::Array(ref events)) = msg.get("events") {
                    println!("events: {:?}", events);
                    let new_events = events.iter().filter_map(|e| {
                        match e {
                            &Json::String(ref e) => Some(e),
                            _ => None,
                        }
                    });
                    let mut events = self.events.borrow_mut();
                    for new_event in new_events {
                        if !events.iter().any(|e| e == new_event) {
                            events.push(new_event.to_owned());
                        }
                    }
                    println!("{:?}", &events[..]);
                }
                ActorMessageStatus::Processed
            },
            "__unregisterEventNotifications" => {
                if let Some(&Json::Array(ref events)) = msg.get("events") {
                    println!("events: {:?}", events);
                    let events_to_remove = events.iter().filter_map(|e| {
                        match e {
                            &Json::String(ref e) => Some(e),
                            _ => None,
                        }
                    });
                    let mut events = self.events.borrow_mut();
                    for event in events_to_remove {
                        if let Some(index) = events.iter().position(|e| e == event) {
                            events.remove(index);
                        }
                    }
                    println!("{:?}", &events[..]);
                }
                ActorMessageStatus::Processed
            },
            _ => ActorMessageStatus::Ignored,
        })
    }
}

impl ProfilerActor {
    pub fn new(name: String) -> ProfilerActor {
        ProfilerActor {
            name: name,
            events: RefCell::new(vec![]),
        }
    }
}
