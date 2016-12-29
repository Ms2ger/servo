/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// XXX
#![allow(missing_docs)]

use core::nonzero::NonZero;
use dom::bindings::js::Root;
use dom::globalscope::GlobalScope;
use std::cell::Cell;

thread_local!(static ENTRY: Cell<Option<NonZero<*const GlobalScope>>> = Cell::new(None));

/// A str
pub struct AutoEntryScript {
    previous: Option<Root<GlobalScope>>,
}

impl AutoEntryScript {
    /// https://html.spec.whatwg.org/multipage/#prepare-to-run-script
    pub fn new(global: &GlobalScope) -> Self {
        ENTRY.with(|entry| {
            let previous = entry.get();
            entry.set(Some(unsafe { NonZero::new(global as *const _) }));
            AutoEntryScript {
                previous: previous.map(Root::new),
            }
        })
    }
}

impl Drop for AutoEntryScript {
    /// https://html.spec.whatwg.org/multipage/#clean-up-after-running-script
    fn drop(&mut self) {
        ENTRY.with(|entry| {
            entry.set(self.previous.as_mut().map(|previous| unsafe { NonZero::new(&**previous as *const _) }));
        })
    }
}

pub fn get_entry_global() -> NonZero<*const GlobalScope> {
    ENTRY.with(|entry| entry.get()).unwrap()
}
