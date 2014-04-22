/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::trace::Traceable;
use dom::bindings::utils::{Reflectable, global_object_for_js_object};
use js::jsapi::{JSContext, JSObject, JS_WrapObject, JS_ObjectIsCallable};
use js::jsapi::JS_GetProperty;
use js::jsval::{JSVal, UndefinedValue};

use std::ptr;

use serialize::{Encodable, Encoder};

pub enum ExceptionHandling {
    // Report any exception and don't throw it to the caller code.
    ReportExceptions,
    // Throw an exception to the caller code if the thrown exception is a
    // binding object for a DOMError from the caller's scope, otherwise report
    // it.
    RethrowContentExceptions,
    // Throw any exception to the caller code.
    RethrowExceptions
}

#[deriving(Clone,Eq,Encodable)]
pub struct CallbackFunction {
    object: CallbackObject
}

impl CallbackFunction {
    pub fn new(callback: *JSObject) -> CallbackFunction {
        CallbackFunction {
            object: CallbackObject {
                callback: Traceable::new(callback)
            }
        }
    }
}

#[deriving(Clone,Eq,Encodable)]
pub struct CallbackInterface {
    object: CallbackObject
}

#[deriving(Clone,Eq,Encodable)]
struct CallbackObject {
    callback: Traceable<*JSObject>,
}

pub trait CallbackContainer {
    fn callback(&self) -> *JSObject;
}

impl CallbackContainer for CallbackInterface {
    fn callback(&self) -> *JSObject {
        *self.object.callback.deref()
    }
}

impl CallbackContainer for CallbackFunction {
    fn callback(&self) -> *JSObject {
        *self.object.callback.deref()
    }
}

impl CallbackInterface {
    pub fn new(callback: *JSObject) -> CallbackInterface {
        CallbackInterface {
            object: CallbackObject {
                callback: Traceable::new(callback)
            }
        }
    }

    pub fn GetCallableProperty(&self, cx: *JSContext, name: &str) -> Result<JSVal, ()> {
        let mut callable = UndefinedValue();
        unsafe {
            if name.to_c_str().with_ref(|name| JS_GetProperty(cx, self.callback(), name, &mut callable as *mut JSVal as *JSVal)) == 0 {
                return Err(());
            }

            if !callable.is_object() ||
               JS_ObjectIsCallable(cx, callable.to_object()) == 0 {
                //ThrowErrorMessage(cx, MSG_NOT_CALLABLE, description.get());
                return Err(());
            }
        }
        Ok(callable)
    }
}

pub fn GetJSObjectFromCallback<T: CallbackContainer>(callback: &T) -> *JSObject {
    callback.callback()
}

pub fn WrapCallThisObject<T: 'static + CallbackContainer + Reflectable>(cx: *JSContext,
                                                                        _scope: *JSObject,
                                                                        p: Box<T>) -> *JSObject {
    let obj = GetJSObjectFromCallback(p);
    assert!(obj.is_not_null());

    unsafe {
        if JS_WrapObject(cx, &obj) == 0 {
            return ptr::null();
        }
    }

    return obj;
}

pub struct CallSetup {
    pub cx: *JSContext,
    pub handling: ExceptionHandling
}

impl CallSetup {
    pub fn new<T: CallbackContainer>(callback: &T, handling: ExceptionHandling) -> CallSetup {
        let win = global_object_for_js_object(callback.callback());
        let cx = win.get().get_cx();
        CallSetup {
            cx: cx,
            handling: handling
        }
    }

    pub fn GetContext(&self) -> *JSContext {
        self.cx
    }
}
