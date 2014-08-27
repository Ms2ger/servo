/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use js::glue::ProxyTraps;
use js::jsapi::{JSContext, JSObject, jsid, JSPropertyDescriptor};

use std::ptr;

fn ReportError(cx: *mut JSContext, message: &str) {
    use js::glue::ReportError;
    message.with_c_str(|string| {
        unsafe { ReportError(cx, string) };
    });
}

extern fn define_property(cx: *mut JSContext, _wrapper: *mut JSObject,
                          _id: jsid, _desc: *mut JSPropertyDescriptor)
                          -> bool {
    ReportError(cx, "Permission denied to define property on cross-origin object");
    return false;
}

extern fn delete(cx: *mut JSContext, _wrapper: *mut JSObject, _id: jsid,
                 _bp: *mut bool) -> bool {
    ReportError(cx, "Permission denied to delete property on cross-origin object");
    return false;
}

extern fn get_prototype_of(_cx: *mut JSContext,
                           _wrapper: *mut JSObject,
                           proto: *mut *mut JSObject)
                           -> bool {
    unsafe {
        *proto = ptr::mut_null();
    }
    true
}

pub static proxy_handler: ProxyTraps = ProxyTraps {
    getPropertyDescriptor: None,
    getOwnPropertyDescriptor: None,
    defineProperty: Some(define_property),
    getOwnPropertyNames: 0 as *const u8,
    delete_: Some(delete),
    enumerate: 0 as *const u8,

    has: None,
    hasOwn: None,
    get: None,
    set: None,
    keys: 0 as *const u8,
    iterate: None,

    call: None,
    construct: None,
    nativeCall: 0 as *const u8,
    hasInstance: None,
    typeOf: None,
    objectClassIs: None,
    obj_toString: None,
    fun_toString: None,
    //regexp_toShared: 0 as *u8,
    defaultValue: None,
    iteratorNext: None,
    finalize: None,
    getElementIfPresent: None,
    getPrototypeOf: Some(get_prototype_of),
    trace: None
};
