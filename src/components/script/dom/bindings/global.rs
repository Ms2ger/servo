/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::js::{JS, JSRef};
use dom::bindings::utils::{Reflectable, Reflector};
use dom::window::Window;

use js::jsapi::JSContext;

pub enum GlobalRef<'a> {
    Window(JSRef<'a, Window>),
    Worker,
}

#[deriving(Encodable)]
pub enum GlobalField {
    WindowField(JS<Window>),
    WorkerField,
}

impl<'a> GlobalRef<'a> {
    pub fn get_cx(&self) -> *mut JSContext {
        match *self {
            Window(window) => window.get_cx(),
            Worker => fail!("NYI"),
        }
    }
}

impl<'a> Reflectable for GlobalRef<'a> {
    fn reflector<'b>(&'b self) -> &'b Reflector {
        match *self {
            Window(ref window) => window.reflector(),
            Worker => fail!("NYI"),
        }
    }
}

impl GlobalField {
    pub fn from_rooted(global: &GlobalRef) -> GlobalField {
        match *global {
            Window(ref window) => WindowField(JS::from_rooted(window)),
            Worker => fail!("NYI"),
        }
    }

    pub fn root(&self) -> ! {
        fail!("NYI")
    }
}
