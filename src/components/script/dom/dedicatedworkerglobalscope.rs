/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::DedicatedWorkerGlobalScopeBinding;
use dom::bindings::codegen::InheritTypes::DedicatedWorkerGlobalScopeDerived;
use dom::bindings::codegen::InheritTypes::{EventTargetCast, WorkerGlobalScopeCast};
use dom::bindings::global::Worker;
use dom::bindings::js::{JSRef, Temporary, RootCollection};
use dom::bindings::trace::Untraceable;
use dom::bindings::utils::{Reflectable, Reflector};
use dom::eventtarget::EventTarget;
use dom::eventtarget::WorkerGlobalScopeTypeId;
use dom::messageevent::MessageEvent;
use dom::worker::TrustedWorkerAddress;
use dom::workerglobalscope::DedicatedGlobalScope;
use dom::workerglobalscope::WorkerGlobalScope;
use script_task::{ScriptTask, ScriptChan, WorkerPostMessage, WorkerRelease};
use script_task::StackRootTLS;

use servo_net::resource_task::{ResourceTask, load_whole_resource};

use servo_util::str::DOMString;

use js::rust::{Cx, JSAutoRequest};

use std::rc::Rc;
use native;
use rustrt::task::TaskOpts;
use url::Url;

#[deriving(Encodable)]
pub struct DedicatedWorkerGlobalScope {
    workerglobalscope: WorkerGlobalScope,
    receiver: Untraceable<Receiver<DOMString>>,
    worker: Untraceable<TrustedWorkerAddress>,
}

impl DedicatedWorkerGlobalScope {
    pub fn new_inherited(worker_url: Url,
                         worker: TrustedWorkerAddress,
                         cx: Rc<Cx>,
                         receiver: Receiver<DOMString>,
                         resource_task: ResourceTask,
                         script_chan: ScriptChan)
                         -> DedicatedWorkerGlobalScope {
        DedicatedWorkerGlobalScope {
            workerglobalscope: WorkerGlobalScope::new_inherited(
                DedicatedGlobalScope, worker_url, cx, resource_task,
                script_chan),
            receiver: Untraceable::new(receiver),
            worker: Untraceable::new(worker),
        }
    }

    pub fn new(worker_url: Url,
               worker: TrustedWorkerAddress,
               cx: Rc<Cx>,
               receiver: Receiver<DOMString>,
               resource_task: ResourceTask,
               script_chan: ScriptChan)
               -> Temporary<DedicatedWorkerGlobalScope> {
        let scope = box DedicatedWorkerGlobalScope::new_inherited(
            worker_url, worker, cx.clone(), receiver, resource_task,
            script_chan);
        DedicatedWorkerGlobalScopeBinding::Wrap(cx.ptr, scope)
    }
}

impl DedicatedWorkerGlobalScope {
    pub fn run_worker_scope(worker_url: Url,
                            worker: TrustedWorkerAddress,
                            receiver: Receiver<DOMString>,
                            resource_task: ResourceTask,
                            script_chan: ScriptChan) {
        let mut task_opts = TaskOpts::new();
        task_opts.name = Some(format!("Web Worker at {}", worker_url).into_maybe_owned());
        native::task::spawn_opts(task_opts, proc() {
            let roots = RootCollection::new();
            let _stack_roots_tls = StackRootTLS::new(&roots);

            let (filename, source) = match load_whole_resource(&resource_task, worker_url.clone()) {
                Err(_) => {
                    println!("error loading script {}", worker_url);
                    return;
                }
                Ok((metadata, bytes)) => {
                    (metadata.final_url, String::from_utf8(bytes).unwrap())
                }
            };

            let (_js_runtime, js_context) = ScriptTask::new_rt_and_cx();
            let global = DedicatedWorkerGlobalScope::new(
                worker_url, worker, js_context.clone(), receiver, resource_task,
                script_chan).root();
            {
                let _ar = JSAutoRequest::new(js_context.ptr);
                match js_context.evaluate_script(
                    global.reflector().get_jsobject(), source, filename.to_str(), 1) {
                    Ok(_) => (),
                    Err(_) => println!("evaluate_script failed")
                }
            }
            global.delayed_release_worker();

            let scope: &JSRef<WorkerGlobalScope> =
                WorkerGlobalScopeCast::from_ref(&*global);
            let target: &JSRef<EventTarget> =
                EventTargetCast::from_ref(&*global);
            loop {
                match global.receiver.recv_opt() {
                    Ok(message) => {
                        MessageEvent::dispatch(target, &Worker(*scope), message);
                        global.delayed_release_worker();
                    },
                    Err(_) => break,
                }
            }
        });
    }
}

trait PrivateDedicatedWorkerGlobalScopeHelpers {
    fn delayed_release_worker(&self);
}

impl<'a> PrivateDedicatedWorkerGlobalScopeHelpers for JSRef<'a, DedicatedWorkerGlobalScope> {
    fn delayed_release_worker(&self) {
        let scope: &JSRef<WorkerGlobalScope> =
            WorkerGlobalScopeCast::from_ref(self);
        let ScriptChan(ref sender) = *scope.script_chan();
        sender.send(WorkerRelease(*self.worker));
    }
}

pub trait DedicatedWorkerGlobalScopeMethods {
    fn PostMessage(&self, message: DOMString);
}

impl<'a> DedicatedWorkerGlobalScopeMethods for JSRef<'a, DedicatedWorkerGlobalScope> {
    fn PostMessage(&self, message: DOMString) {
        let scope: &JSRef<WorkerGlobalScope> =
            WorkerGlobalScopeCast::from_ref(self);
        let ScriptChan(ref sender) = *scope.script_chan();
        sender.send(WorkerPostMessage(*self.worker, message));
    }
}

impl Reflectable for DedicatedWorkerGlobalScope {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.workerglobalscope.reflector()
    }
}

impl DedicatedWorkerGlobalScopeDerived for EventTarget {
    fn is_dedicatedworkerglobalscope(&self) -> bool {
        match self.type_id {
            WorkerGlobalScopeTypeId(DedicatedGlobalScope) => true,
            _ => false
        }
    }
}
