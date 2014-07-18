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
use dom::workerglobalscope::DedicatedGlobalScope;
use dom::workerglobalscope::WorkerGlobalScope;
use script_task::{ScriptTask, ScriptChan};
use script_task::StackRootTLS;

use servo_net::resource_task::{ResourceTask, load_whole_resource};

use servo_util::str::DOMString;

use js::rust::Cx;

use std::rc::Rc;
use native;
use rustrt::task::TaskOpts;
use url::Url;


use dom::workerglobalscope::{ControlMessage, Shutdown};
use std::comm::{Empty, Disconnected};


#[deriving(Encodable)]
pub struct DedicatedWorkerGlobalScope {
    workerglobalscope: WorkerGlobalScope,
    control_receiver: Untraceable<Receiver<ControlMessage>>,
    receiver: Untraceable<Receiver<DOMString>>,
}

impl DedicatedWorkerGlobalScope {
    pub fn new_inherited(worker_url: Url,
                         cx: Rc<Cx>,
                         control_receiver: Receiver<ControlMessage>,
                         receiver: Receiver<DOMString>,
                         resource_task: ResourceTask,
                         script_chan: ScriptChan)
                         -> DedicatedWorkerGlobalScope {
        DedicatedWorkerGlobalScope {
            workerglobalscope: WorkerGlobalScope::new_inherited(
                DedicatedGlobalScope, worker_url, cx, resource_task,
                script_chan),
            control_receiver: Untraceable::new(control_receiver),
            receiver: Untraceable::new(receiver),
        }
    }

    pub fn new(worker_url: Url,
               cx: Rc<Cx>,
               control_receiver: Receiver<ControlMessage>,
               receiver: Receiver<DOMString>,
               resource_task: ResourceTask,
               script_chan: ScriptChan)
               -> Temporary<DedicatedWorkerGlobalScope> {
        let scope = box DedicatedWorkerGlobalScope::new_inherited(
            worker_url, cx.clone(), control_receiver, receiver, resource_task,
            script_chan);
        DedicatedWorkerGlobalScopeBinding::Wrap(cx.ptr, scope)
    }
}

impl DedicatedWorkerGlobalScope {
    pub fn run_worker_scope(worker_url: Url,
                            control_receiver: Receiver<ControlMessage>,
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
                worker_url, js_context.clone(), control_receiver, receiver,
                resource_task, script_chan).root();
            match js_context.evaluate_script(
                global.reflector().get_jsobject(), source, filename.to_str(), 1) {
                Ok(_) => (),
                Err(_) => println!("evaluate_script failed")
            }

            let scope: &JSRef<WorkerGlobalScope> =
                WorkerGlobalScopeCast::from_ref(&*global);
            let target: &JSRef<EventTarget> =
                EventTargetCast::from_ref(&*global);
            loop {
                loop {
                    match global.control_receiver.try_recv() {
                        Ok(Shutdown) => {
                            return;
                        },
                        Err(Empty) => break,
                        Err(Disconnected) => return,
                    }
                }
                match global.receiver.recv_opt() {
                    Ok(message) => {
                        MessageEvent::dispatch(target, &Worker(*scope), message)
                    },
                    Err(_) => return,
                }
            }
        });
    }
}

#[unsafe_destructor]
impl Drop for DedicatedWorkerGlobalScope {
    fn drop(&mut self) {
        /*
        let ScriptChan(ref sender) = *self.workerglobalscope.script_chan();
        let (shutdown_sender, shutdown_receiver) = channel();
        self.shutdown_sender.send(WorkerShutdown(shutdown_sender));
        shutdown_receiver.recv();
        */
    }
}

pub trait DedicatedWorkerGlobalScopeMethods {
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
