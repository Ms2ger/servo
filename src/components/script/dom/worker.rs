/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WorkerBinding;
use dom::bindings::error::{Fallible, Syntax};
use dom::bindings::global::GlobalRef;
use dom::bindings::js::{JSRef, Temporary};
use dom::bindings::trace::Untraceable;
use dom::bindings::utils::{Reflectable, Reflector, reflect_dom_object};
use dom::dedicatedworkerglobalscope::DedicatedWorkerGlobalScope;
use dom::eventtarget::{EventTarget, WorkerTypeId};

use servo_util::str::DOMString;
use servo_util::url::try_parse_url;

use dom::workerglobalscope::{ControlMessage, Shutdown};


#[deriving(Encodable)]
pub struct Worker {
    eventtarget: EventTarget,
    control_sender: Untraceable<Sender<ControlMessage>>,
    sender: Untraceable<Sender<DOMString>>,
}

impl Worker {
    pub fn new_inherited(control_sender: Sender<ControlMessage>,
                         sender: Sender<DOMString>) -> Worker {
        Worker {
            eventtarget: EventTarget::new_inherited(WorkerTypeId),
            control_sender: Untraceable::new(control_sender),
            sender: Untraceable::new(sender),
        }
    }

    pub fn new(global: &GlobalRef,
               control_sender: Sender<ControlMessage>,
               sender: Sender<DOMString>) -> Temporary<Worker> {
        reflect_dom_object(box Worker::new_inherited(control_sender, sender),
                           global,
                           WorkerBinding::Wrap)
    }

    // http://www.whatwg.org/html/#dom-worker
    pub fn Constructor(global: &GlobalRef, scriptURL: DOMString) -> Fallible<Temporary<Worker>> {
        // Step 2-4.
        let worker_url = match try_parse_url(scriptURL.as_slice(), Some(global.get_url())) {
            Ok(url) => url,
            Err(_) => return Err(Syntax),
        };

        let (control_sender, control_receiver) = channel();
        let (sender, receiver) = channel();
        let resource_task = global.resource_task();
        DedicatedWorkerGlobalScope::run_worker_scope(
            worker_url, control_receiver, receiver, resource_task,
            global.script_chan().clone());
        Ok(Worker::new(global, control_sender, sender))
    }
}

pub trait WorkerMethods {
    fn Terminate(&self);
    fn PostMessage(&self, message: DOMString);
}

impl<'a> WorkerMethods for JSRef<'a, Worker> {
    fn Terminate(&self) {
        self.control_sender.send(Shutdown);
    }

    fn PostMessage(&self, message: DOMString) {
        self.sender.send(message);
    }
}

impl Reflectable for Worker {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.eventtarget.reflector()
    }
}
