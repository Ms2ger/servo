/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use js::glue::ProxyTraps;
use js::glue::{GetProxyExtra, SetProxyExtra};
use js::jsapi::{JSContext, JSObject, JSPropertyDescriptor, jsid};
use js::jsapi::{JS_GetGlobalForObject, JS_NewObjectWithGivenProto};
use js::jsval::ObjectValue;

use libc;
use std::ptr;

static HOLDER_SLOT: libc::c_uint = 0;

fn get_holder(wrapper: *mut JSObject) -> *mut JSObject {
    GetProxyExtra(wrapper, HOLDER_SLOT).to_object()
}

fn ensure_holder(cx: *mut JSContext, wrapper: *mut JSObject) -> *mut JSObject
{
    match get_holder(wrapper) {
        0 => {
            let global = JS_GetGlobalForObject(cx, wrapper);
            let holder = JS_NewObjectWithGivenProto(cx, ptr::mut_null(),
                                                    ptr::mut_null(), global);
            assert!(holder.is_not_null());
            SetProxyExtra(wrapper, HOLDER_SLOT, ObjectValue(&*holder));
            holder
        }
        holder => holder as *mut _,
    }
}


extern fn get_property_descriptor(cx: *mut JSContext,
                                  wrapper: *mut JSObject,
                                  _id: jsid,
                                  _set: bool,
                                  _desc: *mut JSPropertyDescriptor)
                                  -> bool {
    let holder = ensure_holder(cx, wrapper);
/*
    // Ordering is important here.
    //
    // We first need to call resolveOwnProperty, even before checking the holder,
    // because there might be a new dynamic |own| property that appears and
    // shadows a previously-resolved non-own property that we cached on the
    // holder. This can happen with indexed properties on NodeLists, for example,
    // which are |own| value props.
    //
    // resolveOwnProperty may or may not cache what it finds on the holder,
    // depending on how ephemeral it decides the property is. XPCWN |own|
    // properties generally end up on the holder via NewResolve, whereas
    // NodeList |own| properties don't get defined on the holder, since they're
    // supposed to be dynamic. This means that we have to first check the result
    // of resolveOwnProperty, and _then_, if that comes up blank, check the
    // holder for any cached native properties.
    //
    // Finally, we call resolveNativeProperty, which checks non-own properties,
    // and unconditionally caches what it finds on the holder.

    // Check resolveOwnProperty.
    if (!xpc::DOMXrayTraits::singleton.resolveOwnProperty(cx, *this, wrapper, holder, id, desc))
        return false;

    // Check the holder.
    if (!desc.object() && !JS_GetPropertyDescriptorById(cx, holder, id, desc))
        return false;
    if (desc.object()) {
        desc.object().set(wrapper);
        return true;
    }

    // Nothing in the cache. Call through, and cache the result.
    RootedObject obj(cx, getTargetObject(wrapper));
    if (!XrayResolveNativeProperty(cx, wrapper, holder, id, desc))
        return false;

    MOZ_ASSERT(!desc.object() || desc.object() == wrapper, "What did we resolve this on?");


    if (!desc.object() &&
        id == nsXPConnect::GetRuntimeInstance()->GetStringID(XPCJSRuntime::IDX_TO_STRING))
    {

        JSFunction *toString = JS_NewFunction(cx, XrayToString, 0, 0, wrapper, "toString");
        if (!toString)
            return false;

        desc.object().set(wrapper);
        desc.setAttributes(0);
        desc.setGetter(nullptr);
        desc.setSetter(nullptr);
        desc.value().setObject(*JS_GetFunctionObject(toString));
    }

    // If we still have nothing, we're done.
    if (!desc.object())
        return true;

    if (!JS_DefinePropertyById(cx, holder, id, desc.value(), desc.attributes(),
                               desc.getter(), desc.setter()) ||
        !JS_GetPropertyDescriptorById(cx, holder, id, desc))
    {
        return false;
    }
    MOZ_ASSERT(desc.object());
    desc.object().set(wrapper);
    return true;
*/
/*    if (!SecurityXrayDOM::getPropertyDescriptor(cx, wrapper, id, desc))
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
    }*/
    return true;
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

static proxy_handler: ProxyTraps = ProxyTraps {
    getPropertyDescriptor: Some(get_property_descriptor),
    getOwnPropertyDescriptor: Some(get_property_descriptor),
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

