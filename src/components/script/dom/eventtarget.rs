/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::js::JS;
use dom::bindings::utils::{Reflectable, Reflector};
use dom::bindings::error::{Fallible, InvalidState};
use dom::bindings::codegen::EventListenerBinding;
use self::EventListenerBinding::EventListener;
use dom::event::Event;
use dom::eventdispatcher::dispatch_event;
use dom::node::NodeTypeId;
use dom::virtualmethods::VirtualMethods;
use servo_util::str::DOMString;

use collections::hashmap::HashMap;

#[deriving(Eq,Encodable)]
pub enum ListenerPhase {
    Capturing,
    Bubbling,
}

#[deriving(Eq,Encodable)]
pub enum EventTargetTypeId {
    WindowTypeId,
    NodeTargetTypeId(NodeTypeId)
}

#[deriving(Eq, Encodable)]
pub enum EventListenerType {
    Additive(EventListener),
    Inline(EventListener),
}

impl EventListenerType {
    fn get_listener(&self) -> EventListener {
        match *self {
            Additive(listener) | Inline(listener) => listener
        }
    }
}

#[deriving(Eq,Encodable)]
pub struct EventListenerEntry {
    pub phase: ListenerPhase,
    pub listener: EventListenerType
}

#[deriving(Encodable)]
pub struct EventTarget {
    pub type_id: EventTargetTypeId,
    pub reflector_: Reflector,
    pub handlers: HashMap<DOMString, Vec<EventListenerEntry>>,
}

impl EventTarget {
    pub fn new_inherited(type_id: EventTargetTypeId) -> EventTarget {
        EventTarget {
            type_id: type_id,
            reflector_: Reflector::new(),
            handlers: HashMap::new(),
        }
    }

    pub fn get_listeners(&self, type_: &str) -> Option<Vec<EventListener>> {
        self.handlers.find_equiv(&type_).map(|listeners| {
            listeners.iter().map(|entry| entry.listener.get_listener()).collect()
        })
    }

    pub fn get_listeners_for(&self, type_: &str, desired_phase: ListenerPhase)
        -> Option<Vec<EventListener>> {
        self.handlers.find_equiv(&type_).map(|listeners| {
            let filtered = listeners.iter().filter(|entry| entry.phase == desired_phase);
            filtered.map(|entry| entry.listener.get_listener()).collect()
        })
    }

    pub fn AddEventListener(&mut self,
                            ty: DOMString,
                            listener: Option<EventListener>,
                            capture: bool) {
        for &listener in listener.iter() {
            let entry = self.handlers.find_or_insert_with(ty.clone(), |_| vec!());
            let phase = if capture { Capturing } else { Bubbling };
            let new_entry = EventListenerEntry {
                phase: phase,
                listener: Additive(listener)
            };
            if entry.as_slice().position_elem(&new_entry).is_none() {
                entry.push(new_entry);
            }
        }
    }

    pub fn RemoveEventListener(&mut self,
                               ty: DOMString,
                               listener: Option<EventListener>,
                               capture: bool) {
        for &listener in listener.iter() {
            let mut entry = self.handlers.find_mut(&ty);
            for entry in entry.mut_iter() {
                let phase = if capture { Capturing } else { Bubbling };
                let old_entry = EventListenerEntry {
                    phase: phase,
                    listener: Additive(listener)
                };
                let position = entry.as_slice().position_elem(&old_entry);
                for &position in position.iter() {
                    entry.remove(position);
                }
            }
        }
    }

    pub fn DispatchEvent(&self, abstract_self: &JS<EventTarget>,
                         event: &mut JS<Event>) -> Fallible<bool> {
        self.dispatch_event_with_target(abstract_self, None, event)
    }

    pub fn set_inline_event_listener(&mut self,
                                     ty: DOMString,
                                     listener: Option<EventListener>) {
        let entries = self.handlers.find_or_insert_with(ty, |_| ~[]);
        let idx = entries.iter().position(|&entry| {
            match entry.listener {
                Inline(_) => true,
                _ => false,
            }
        });

        match idx {
            Some(idx) => {
                match listener {
                    Some(listener) => entries[idx].listener = Inline(listener),
                    None => {
                        entries.remove(idx);
                    }
                }
            }
            None => {
                if listener.is_some() {
                    entries.push(EventListenerEntry {
                        phase: Capturing, //XXXjdm no idea when inline handlers should run
                        listener: Inline(listener.unwrap()),
                    });
                }
            }
        }
    }

    pub fn get_inline_event_listener(&self, ty: DOMString) -> Option<EventListener> {
        let entries = self.handlers.find(&ty);
        entries.and_then(|entries| entries.iter().find(|entry| {
            match entry.listener {
                Inline(_) => true,
                _ => false,
            }
        }).map(|entry| entry.listener.get_listener()))
    }

    pub fn dispatch_event_with_target(&self,
                                      abstract_self: &JS<EventTarget>,
                                      abstract_target: Option<JS<EventTarget>>,
                                      event: &mut JS<Event>) -> Fallible<bool> {
        if event.get().dispatching || !event.get().initialized {
            return Err(InvalidState);
        }
        Ok(dispatch_event(abstract_self, abstract_target, event))
    }
}

impl Reflectable for EventTarget {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        &self.reflector_
    }

    fn mut_reflector<'a>(&'a mut self) -> &'a mut Reflector {
        &mut self.reflector_
    }
}

impl VirtualMethods for JS<EventTarget> {
    fn super_type(&self) -> Option<~VirtualMethods:> {
        None
    }
}
