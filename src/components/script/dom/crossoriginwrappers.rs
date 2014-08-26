/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use js::jsapi::{JSContext, JSObject, JSBool};
use js::glue::ProxyTraps;

use std::ptr;

extern fn get_property_descriptor(cx: *mut JSContext,
                                  wrapper: *mut JSObject,
                                  id: jsid,
                                  _: JSbool,
                                  desc: *mut JSPropertyDescriptor)
                                  -> JSBool {
    if (!SecurityXrayDOM::getPropertyDescriptor(cx, wrapper, id, desc))
        return false;
    if (desc.object()) {
        // All properties on cross-origin DOM objects are |own|.
        desc.object().set(wrapper);

        // All properties on cross-origin DOM objects are non-enumerable and
        // "configurable". Any value attributes are read-only.
        desc.attributesRef() &= ~JSPROP_ENUMERATE;
        desc.attributesRef() &= ~JSPROP_PERMANENT;
        if (!desc.getter() && !desc.setter())
            desc.attributesRef() |= JSPROP_READONLY;
    }
    return true;
}


extern fn get_prototype_of(_cx: *mut JSContext,
                           _wrapper: *mut JSObject,
                           proto: *mut *mut JSObject)
                           -> JSBool {
    unsafe {
        *proto = ptr::mut_null();
    }
    1
}

static proxy_handler: ProxyTraps = ProxyTraps {
    getPropertyDescriptor: None,
    getOwnPropertyDescriptor: None,
    defineProperty: None,
    getOwnPropertyNames: 0 as *const u8,
    delete_: None,
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

