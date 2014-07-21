/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WorkerBinding;
use dom::bindings::codegen::InheritTypes::EventTargetCast;
use dom::bindings::error::{Fallible, Syntax};
use dom::bindings::global::{GlobalRef, GlobalField};
use dom::bindings::js::{JS, JSRef, Temporary};
use dom::bindings::trace::{Traceable, Untraceable};
use dom::bindings::utils::{Reflectable, Reflector, reflect_dom_object};
use dom::dedicatedworkerglobalscope::DedicatedWorkerGlobalScope;
use dom::eventtarget::{EventTarget, WorkerTypeId};
use dom::messageevent::MessageEvent;

use servo_util::str::DOMString;
use servo_util::url::try_parse_url;

use js::jsapi::{JS_AddObjectRoot, JS_RemoveObjectRoot, JSContext};

use libc::c_void;
use std::cell::Cell;

pub struct TrustedWorkerAddress(pub *c_void);

#[deriving(Encodable)]
pub struct Worker {
    eventtarget: EventTarget,
    refcount: Traceable<Cell<uint>>,
    global: GlobalField,
    sender: Untraceable<Sender<DOMString>>,
}

impl Worker {
    pub fn new_inherited(global: &GlobalRef, sender: Sender<DOMString>) -> Worker {
        Worker {
            eventtarget: EventTarget::new_inherited(WorkerTypeId),
            refcount: Traceable::new(Cell::new(0)),
            global: GlobalField::from_rooted(global),
            sender: Untraceable::new(sender),
        }
    }

    pub fn new(global: &GlobalRef, sender: Sender<DOMString>) -> Temporary<Worker> {
        reflect_dom_object(box Worker::new_inherited(global, sender),
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

        let (sender, receiver) = channel();
        let worker = Worker::new(global, sender).root();
        let worker_ref = worker.addref();

        let resource_task = global.resource_task();
        DedicatedWorkerGlobalScope::run_worker_scope(
            worker_url, worker_ref, receiver, resource_task,
            global.script_chan().clone());
        Ok(Temporary::from_rooted(&*worker))
    }

    pub fn handle_message(address: TrustedWorkerAddress, message: DOMString) {
        let worker = unsafe { JS::from_trusted_worker_address(address).root() };

        let target: &JSRef<EventTarget> = EventTargetCast::from_ref(&*worker);
        let global = worker.global.root();
        MessageEvent::dispatch(target, &global.root_ref(), message);
    }

    pub fn handle_release(address: TrustedWorkerAddress) {
        let worker = unsafe { JS::from_trusted_worker_address(address).root() };
        worker.release();
    }
}

impl Worker {
    // Creates a trusted address to the object, and roots it. Always pair this with a release()
    pub fn addref(&self) -> TrustedWorkerAddress {
        let refcount = self.refcount.deref().get();
        if refcount == 0 {
            unsafe {
                JS_AddObjectRoot(self.global.root().root_ref().get_cx(), self.reflector().rootable());
            }
        }
        self.refcount.set(refcount + 1);
        TrustedWorkerAddress(self as *Worker as *c_void)
    }

    #[inline(never)]
    fn check_cx(cx: *mut JSContext) {
        use js::jsapi::{JS_AbortIfWrongThread, JS_GetRuntime};
        println!("Cx: {:p}", cx);
        unsafe {
            let rt = JS_GetRuntime(cx);
            println!("Rt: {:p}", rt);
            JS_AbortIfWrongThread(rt);
        }
    }

    pub fn release(&self) {
        Worker::check_cx(self.global.root().root_ref().get_cx());
        let refcount = self.refcount.get();
        assert!(refcount > 0)
        self.refcount.set(refcount - 1);
        if refcount == 1 {
            unsafe {
                JS_RemoveObjectRoot(self.global.root().root_ref().get_cx(), self.reflector().rootable());
            }
        }
    }
}

pub trait WorkerMethods {
    fn PostMessage(&self, message: DOMString);
}

impl<'a> WorkerMethods for JSRef<'a, Worker> {
    fn PostMessage(&self, message: DOMString) {
        self.addref();
        self.sender.send(message);
    }
}

impl Reflectable for Worker {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.eventtarget.reflector()
    }
}
