/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Supports writing a trace file created during each layout scope
//! that can be viewed by an external tool to make layout debugging easier.

// for thread_local
#![allow(unsafe_code)]

use flow;
use flow_ref::FlowRef;
use rustc_serialize::json;
use std::borrow::ToOwned;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering};

thread_local!(static STATE_KEY: RefCell<Option<State>> = RefCell::new(None));

static DEBUG_ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct Scope;

#[macro_export]
macro_rules! layout_debug_scope(
    ($($arg:tt)*) => (
        if cfg!(debug_assertions) {
            layout_debug::Scope::new(format!($($arg)*))
        } else {
            layout_debug::Scope
        }
    )
);

#[derive(RustcEncodable)]
struct ScopeData {
    name: String,
    pre: String,
    post: String,
    children: Vec<Box<ScopeData>>,
}

impl ScopeData {
    fn new(name: String, pre: String) -> ScopeData {
        ScopeData {
            name: name,
            pre: pre,
            post: String::new(),
            children: vec!(),
        }
    }
}

struct State {
    flow_root: FlowRef,
    scope_stack: Vec<Box<ScopeData>>,
}

/// A layout debugging scope. The entire state of the flow tree
/// will be output at the beginning and end of this scope.
impl Scope {
    pub fn new(name: String) -> Scope {
        Scope
    }
}

#[cfg(debug_assertions)]
impl Drop for Scope {
    fn drop(&mut self) {
    }
}

/// Generate a unique ID. This is used for items such as Fragment
/// which are often reallocated but represent essentially the
/// same data.
pub fn generate_unique_debug_id() -> u16 {
    DEBUG_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u16
}

/// Begin a layout debug trace. If this has not been called,
/// creating debug scopes has no effect.
pub fn begin_trace(flow_root: FlowRef) {
    assert!(STATE_KEY.with(|ref r| r.borrow().is_none()));
}

/// End the debug layout trace. This will write the layout
/// trace to disk in the current directory. The output
/// file can then be viewed with an external tool.
pub fn end_trace() {
}
