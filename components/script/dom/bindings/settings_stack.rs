/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// XXX
#![allow(missing_docs)]

use dom::bindings::js::{JS, Root};
use dom::globalscope::GlobalScope;
use std::cell::RefCell;
use js::jsapi::JSTracer;
use dom::bindings::trace::JSTraceable;

thread_local!(static STACK: RefCell<Vec<StackEntry>> = RefCell::new(Vec::new()));

#[derive(PartialEq, Eq)]
#[derive(JSTraceable)]
enum StackEntryKind {
    Incumbent,
    Entry,
}

#[allow(unrooted_must_root)]
#[derive(JSTraceable)]
struct StackEntry {
    global: JS<GlobalScope>,
    kind: StackEntryKind,
}

pub unsafe fn trace(tracer: *mut JSTracer) {
    STACK.with(|stack| {
        stack.borrow().trace(tracer);
    })
}

pub struct AutoEntryScript {
}

impl AutoEntryScript {
    /// https://html.spec.whatwg.org/multipage/#prepare-to-run-script
    pub fn new(global: &GlobalScope) -> Self {
        STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.push(StackEntry {
                global: JS::from_ref(global),
                kind: StackEntryKind::Entry,
            });
            AutoEntryScript {
            }
        })
    }
}

impl Drop for AutoEntryScript {
    /// https://html.spec.whatwg.org/multipage/#clean-up-after-running-script
    fn drop(&mut self) {
        STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.pop().unwrap();
        })
    }
}

fn get(kind: StackEntryKind) -> Root<GlobalScope> {
    STACK.with(|stack| {
        stack.borrow()
             .iter()
             .rev()
             .find(|entry| entry.kind == kind)
             .map(|entry| Root::from_ref(&*entry.global))
    }).unwrap()
}

pub fn entry_global() -> Root<GlobalScope> {
    get(StackEntryKind::Entry)
}

pub fn incumbent_global() -> Root<GlobalScope> {
    get(StackEntryKind::Incumbent)
}
