/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

///! Utilities for the implementation of JSAPI proxy handlers.

/*<<<<<<< HEAD
use dom::bindings::utils::is_dom_proxy;
use js::jsapi::{JSContext, jsid, JSPropertyDescriptor, JSObject, JSString, jschar};
use js::jsapi::{JS_GetPropertyDescriptorById, JS_NewUCString, JS_malloc, JS_free};
use js::jsapi::{JS_DefinePropertyById, JS_NewObjectWithGivenProto};
use js::jsapi::{JS_ReportErrorFlagsAndNumber, JS_StrictPropertyStub};
=======*/
use dom::bindings::utils::{object_handle, mut_object_handle, mut_handle};
use js::jsapi::{JSContext, JSPropertyDescriptor, JSObject, JSString, jschar};
use js::jsapi::{JS_GetPropertyDescriptorById, JS_NewUCString, JS_malloc, JS_free};
use js::jsapi::{JS_DefinePropertyById, JS_NewObjectWithGivenProto, MutableHandle};
use js::jsapi::{JS_StrictPropertyStub, JSHandleObject, JSHandleId};
use js::jsapi::{JSREPORT_WARNING, JSREPORT_STRICT, JSREPORT_STRICT_MODE_ERROR};
use js::jsapi::{JS_ReportErrorFlagsAndNumber, JS_DeletePropertyById2};
use js::jsval::{ObjectValue};
//>>>>>>> b93d695... Build with upgraded SpiderMonkey.
use js::glue::GetProxyExtra;
use js::glue::{GetObjectProto, GetObjectParent, SetProxyExtra, GetProxyHandler};
use js::glue::InvokeGetOwnPropertyDescriptor;
use js::glue::RUST_js_GetErrorMessage;
use js::{JSPROP_GETTER, JSPROP_ENUMERATE, JSPROP_READONLY, JSRESOLVE_QUALIFIED};

use libc;
use std::mem;
use std::ptr;
use std::str;
use std::mem::size_of;

static JSPROXYSLOT_EXPANDO: u32 = 0;

pub extern fn getPropertyDescriptor(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId,
                                    mut desc: MutableHandle<JSPropertyDescriptor>, flags: u32) -> bool {
  unsafe {
    let handler = GetProxyHandler(*proxy);
    if !InvokeGetOwnPropertyDescriptor(handler, cx, proxy, id, mut_handle(desc.deref_mut()), flags) {
        return false;
    }
    if desc.obj.is_not_null() {
        return true;
    }

    //let proto = JS_GetPrototype(proxy);
    let mut proto = ptr::mut_null();
    assert!(GetObjectProto(cx, proxy, mut_object_handle(&mut proto)));
    if proto.is_null() {
        desc.obj = ptr::mut_null();
        return true;
    }

    JS_GetPropertyDescriptorById(cx, object_handle(&proto), id, JSRESOLVE_QUALIFIED, desc)
  }
}

pub fn defineProperty_(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId,
                       desc: MutableHandle<JSPropertyDescriptor>) -> bool {
    static JSMSG_GETTER_ONLY: libc::c_uint = 160;

    unsafe {
        //FIXME: Workaround for https://github.com/mozilla/rust/issues/13385
        let setter: *const libc::c_void = mem::transmute(desc.setter);
        let setter_stub: *const libc::c_void = mem::transmute(JS_StrictPropertyStub);
        if (desc.attrs & JSPROP_GETTER) != 0 && setter == setter_stub {
            return JS_ReportErrorFlagsAndNumber(cx,
                                                JSREPORT_WARNING | JSREPORT_STRICT |
                                                JSREPORT_STRICT_MODE_ERROR,
                                                Some(RUST_js_GetErrorMessage), ptr::mut_null(),
                                                JSMSG_GETTER_ONLY);
        }

        let expando = EnsureExpandoObject(cx, *proxy);
        if expando.is_null() {
            return false;
        }

        return JS_DefinePropertyById(cx, expando, *id, desc.value,
                                     desc.getter, desc.setter, desc.attrs);
    }
}

pub extern fn defineProperty(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId,
                             desc: MutableHandle<JSPropertyDescriptor>) -> bool {
    defineProperty_(cx, proxy, id, desc)
}

pub extern fn delete_(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId,
                      bp: *mut bool) -> bool {
    unsafe {
        let expando = EnsureExpandoObject(cx, *proxy);
        expando.is_not_null() && JS_DeletePropertyById2(cx, object_handle(&expando), id, bp)
    }
}

pub fn _obj_toString(cx: *mut JSContext, className: *const libc::c_char) -> *mut JSString {
  unsafe {
    let name = str::raw::from_c_str(className);
    let nchars = "[object ]".len() + name.len();
    let chars = JS_malloc(cx, (nchars + 1) as libc::size_t * (size_of::<jschar>() as libc::size_t)) as *mut jschar;
    if chars.is_null() {
        return ptr::mut_null();
    }

    let result = format!("[object {}]", name);
    let result = result.as_slice();
    for (i, c) in result.chars().enumerate() {
      *chars.offset(i as int) = c as jschar;
    }
    *chars.offset(nchars as int) = 0;
    let jsstr = JS_NewUCString(cx, chars, nchars as libc::size_t);
    if jsstr.is_null() {
        JS_free(cx, chars as *mut libc::c_void);
    }
    jsstr
  }
}

pub fn GetExpandoObject(obj: *mut JSObject) -> *mut JSObject {
    unsafe {
        //XXXjdm it would be nice to assert that obj's class is a proxy class
        let val = GetProxyExtra(obj, JSPROXYSLOT_EXPANDO);
        if val.is_undefined() {
            ptr::mut_null()
        } else {
            val.to_object()
        }
    }
}

pub fn EnsureExpandoObject(cx: *mut JSContext, obj: *mut JSObject) -> *mut JSObject {
    unsafe {
        //XXXjdm it would be nice to assert that obj's class is a proxy class
        let mut expando = GetExpandoObject(obj);
        if expando.is_null() {
            expando = JS_NewObjectWithGivenProto(cx, ptr::null(),
                                                 object_handle(&ptr::mut_null()),
                                                 object_handle(&GetObjectParent(obj)));
            if expando.is_null() {
                return ptr::mut_null();
            }

            SetProxyExtra(obj, JSPROXYSLOT_EXPANDO, ObjectValue(&*expando));
        }
        return expando;
    }
}

pub fn FillPropertyDescriptor(desc: &mut JSPropertyDescriptor, obj: *mut JSObject, readonly: bool) {
    desc.obj = obj;
    desc.attrs = if readonly { JSPROP_READONLY } else { 0 } | JSPROP_ENUMERATE;
    desc.getter = None;
    desc.setter = None;
}
