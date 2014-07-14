/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::js::{JS, JSRef, Root};
use dom::bindings::utils::{Reflectable, Reflector};
use dom::window::Window;

use js::jsapi::JSContext;

//#[deriving(Clone)]
pub enum GlobalRef<'a, 'b> {
    Window(&'a JSRef<'b, Window>),
    Worker,
}
/*
            impl <'a, 'b> ::std::clone::Clone for GlobalRef<'a, 'b> {
                #[inline]
                fn clone(&self) -> GlobalRef<'a, 'b> {
                    match *self {
                        Window(r) => Window(r),
                        Worker => Worker
                    }
                }
            }
*/

pub struct GlobalRoot<'a, 'b> {
    global_ref: GlobalRef<'a, 'b>,
}

#[deriving(Encodable)]
pub enum GlobalField {
    WindowField(JS<Window>),
    WorkerField,
}

impl<'a, 'b> GlobalRef<'a, 'b> {
    pub fn get_cx(&self) -> *mut JSContext {
        match *self {
            Window(ref window) => window.get_cx(),
            Worker => fail!("NYI"),
        }
    }

    pub fn as_window<'c>(&'c self) -> &'c JSRef<'c, Window> {
        match *self {
            Window(ref window) => *window,
            Worker => fail!("NYI"),
        }
    }
}

impl<'a, 'b> Reflectable for GlobalRef<'a, 'b> {
    fn reflector<'c>(&'c self) -> &'c Reflector {
        match *self {
            Window(ref window) => window.reflector(),
            Worker => fail!("NYI"),
        }
    }
}

impl<'a, 'b> GlobalRoot<'a, 'b> {
    pub fn root_ref<'c>(&'c self) -> GlobalRef<'c, 'c> {
        self.global_ref
    }
}

impl<'a, 'b, 'c> Deref<GlobalRef<'a, 'a>> for GlobalRoot<'a, 'a> {
    fn deref<'d>(&'d self) -> &'d GlobalRef<'a, 'a> {
        &self.global_ref
    }
}

impl GlobalField {
    pub fn from_rooted(global: &GlobalRef) -> GlobalField {
        match *global {
            Window(ref window) => WindowField(JS::from_rooted(*window)),
            Worker => fail!("NYI"),
        }
    }

    pub fn root(&self) -> GlobalRoot {
        fail!("NYI")
    }
}
