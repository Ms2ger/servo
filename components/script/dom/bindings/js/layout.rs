/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! This module defines the `unsafe_get` function on `LayoutJS<T>`.
//!
//! The function is defined in this module to prevent exposing it outside this
//! crate.

use dom::bindings::js::LayoutJS;
use dom::bindings::utils::Reflectable;

pub trait LayoutJSInternals<T> {
    /// Returns an unsafe pointer to the interior of this JS object. This must
    /// not be exposed to layout.
    unsafe fn unsafe_get(&self) -> *const T;
}

impl<T: Reflectable> LayoutJS<T> {
    unsafe fn unsafe_get(&self) -> *const T {
        *self.ptr
    }
}


