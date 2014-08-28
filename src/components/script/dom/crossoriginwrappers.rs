/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::utils::{NativeProperties, NativePropertyHooks};

use js::glue::ProxyTraps;
use js::glue::{GetProxyExtra, SetProxyExtra, UnwrapObject};
use js::jsapi::{JSContext, JSObject, JSPropertyDescriptor, jsid};
use js::jsapi::{JS_GetGlobalForObject, JS_NewObjectWithGivenProto};
use js::jsval::ObjectValue;
use js::rust::with_compartment;

use libc::c_uint;
use std::ptr;

static HOLDER_SLOT: c_uint = 0;

unsafe fn get_holder(wrapper: *mut JSObject) -> *mut JSObject {
    GetProxyExtra(wrapper, HOLDER_SLOT).to_object()
}

unsafe fn ensure_holder(cx: *mut JSContext, wrapper: *mut JSObject) -> *mut JSObject {
    let holder = get_holder(wrapper);
    if holder.is_not_null() {
        return holder;
    }

    let global = JS_GetGlobalForObject(cx, wrapper);
    let holder = JS_NewObjectWithGivenProto(cx, ptr::mut_null(),
                                            ptr::mut_null(), global);
    assert!(holder.is_not_null());
    SetProxyExtra(wrapper, HOLDER_SLOT, ObjectValue(&*holder));
    return holder;
}

unsafe fn get_target_object(wrapper: *mut JSObject) -> *mut JSObject {
    UnwrapObject(wrapper, /* stopAtOuter = */ 0, ptr::mut_null())
}

unsafe fn get_expando_object(_cx: *mut JSContext, _target: *mut JSObject,
                             _consumer: *mut JSObject) -> *mut JSObject {
    // TODO: implement.
    ptr::mut_null()
}

struct NativePropertyHooksIterator {
    current: &'static NativePropertyHooks,
}

impl Iterator<&'static NativePropertyHooks> for NativePropertyHooksIterator {
    fn next(&mut self) -> Option<&'static NativePropertyHooks> {
        let proto = self.current.proto_hooks;
        match proto {
            Some(proto) => self.current = proto,
            _ => (),
        }
        proto
    }
}

unsafe fn get_native_property_hooks(object: *mut JSObject)
                                    -> NativePropertyHooksIterator {
    use dom::bindings::utils::get_dom_class;
    NativePropertyHooksIterator {
        current: get_dom_class(object).unwrap().native_hooks
    }
}

unsafe fn XrayResolveProperty(cx: *mut JSContext, wrapper: *mut JSObject,
                              _obj: *mut JSObject, id: jsid,
                              desc: &mut JSPropertyDescriptor,
                              native_properties: &'static NativeProperties) -> bool {
    use dom::bindings::utils::jsid_to_str;
    use js::{JSPROP_ENUMERATE, JSPROP_PERMANENT, JSPROP_READONLY};

    let name = jsid_to_str(cx, id);
/*
  let methods = nativeProperties->methods;

  if (methods) {
    if (!XrayResolveMethod(cx, wrapper, obj, id, methods, methodIds,
                           methodSpecs, desc)) {
      return false;
    }
    if (desc.object()) {
      return true;
    }
  }

  if (nativeProperties->attributes) {
    if (!XrayResolveAttribute(cx, wrapper, obj, id,
                              nativeProperties->attributes,
                              nativeProperties->attributeIds,
                              nativeProperties->attributeSpecs, desc)) {
      return false;
    }
    if (desc.object()) {
      return true;
    }
  }
*/
    let constant = native_properties.consts.and_then(|constants| {
        constants.iter().find(|spec| name.as_slice() == spec.get_name())
    });
    match constant {
        Some(constant) => {
            desc.attrs = JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT;
            desc.obj = wrapper;
            desc.value = constant.get_value();
            return true;
        },
        None => (),
    }

    return true;
}

unsafe fn DoXrayResolveNativeProperty(cx: *mut JSContext, wrapper: *mut JSObject,
                                      native_property_hooks: &'static NativePropertyHooks,
                                      obj: *mut JSObject, id: jsid,
                                      desc: &mut JSPropertyDescriptor) -> bool
{
/*
  if (IdEquals(id, "constructor")) {
    return nativePropertyHooks->mConstructorID == constructors::id::_ID_Count ||
           ResolvePrototypeOrConstructor(cx, wrapper, obj,
                                         nativePropertyHooks->mConstructorID,
                                         0, desc);
  }
*/

  let native_properties = native_property_hooks.native_properties;

  return XrayResolveProperty(cx, wrapper, obj, id, desc, native_properties);
}

unsafe fn XrayResolveNativeProperty(cx: *mut JSContext, wrapper: *mut JSObject,
                                    obj: *mut JSObject, id: jsid,
                                    desc: &mut JSPropertyDescriptor) -> bool {
    for nativePropertyHooks in get_native_property_hooks(obj) {
        if !DoXrayResolveNativeProperty(cx, wrapper, nativePropertyHooks,
                                        obj, id, desc) {
            return false;
        }

        if desc.obj.is_not_null() {
            return true;
        }
    }

    return true;
}


unsafe fn resolve_own_property(cx: *mut JSContext, wrapper: *mut JSObject,
                               _holder: *mut JSObject, id: jsid, flags: c_uint,
                               desc: &mut JSPropertyDescriptor) -> bool {
    use js::jsapi::JS_GetPropertyDescriptorById;
    use js::jsfriendapi::JS_WrapPropertyDescriptor;

    desc.obj = ptr::mut_null();
    let target = get_target_object(wrapper);
    let expando = get_expando_object(cx, target, wrapper);

    // Check for expando properties first. Note that the expando object lives
    // in the target compartment.
    let mut found = false;
    if expando.is_not_null() {
        with_compartment(cx, expando, || {
            assert!(JS_GetPropertyDescriptorById(cx, expando, id, flags, desc) != 0);
            found = desc.obj.is_not_null();
        })
    }
/*
    // Next, check for ES builtins.
    if (!found && JS_IsGlobalObject(target)) {
        JSProtoKey key = JS_IdToProtoKey(cx, id);
        JSAutoCompartment ac(cx, target);
        if (key != JSProto_Null) {
            MOZ_ASSERT(key < JSProto_LIMIT);
            RootedObject constructor(cx);
            if (!JS_GetClassObject(cx, key, &constructor))
                return false;
            MOZ_ASSERT(constructor);
            desc.value().set(ObjectValue(*constructor));
            found = true;
        } else if (id == GetRTIdByIndex(cx, XPCJSRuntime::IDX_EVAL)) {
            RootedObject eval(cx);
            if (!js::GetOriginalEval(cx, target, &eval))
                return false;
            desc.value().set(ObjectValue(*eval));
            found = true;
        }
    }
*/
    if found {
        if JS_WrapPropertyDescriptor(cx, desc) == 0 {
            return false;
        }

        // Pretend the property lives on the wrapper.
        desc.obj = wrapper;
        return true;
    }
/*
    RootedObject obj(cx, getTargetObject(wrapper));
    if (!XrayResolveOwnProperty(cx, wrapper, obj, id, desc))
        return false;

    MOZ_ASSERT(!desc.object() || desc.object() == wrapper, "What did we resolve this on?");
*/
    return true;
}

extern fn get_property_descriptor(cx: *mut JSContext,
                                  wrapper: *mut JSObject,
                                  id: jsid,
                                  set: bool,
                                  desc: *mut JSPropertyDescriptor)
                                  -> bool {
    use js::{JSRESOLVE_ASSIGNING, JSRESOLVE_QUALIFIED};
    use js::{JSPROP_ENUMERATE, JSPROP_PERMANENT, JSPROP_READONLY};
    use js::jsapi::JS_GetPropertyDescriptorById;

    let flags = (if set { JSRESOLVE_ASSIGNING } else { 0 }) | JSRESOLVE_QUALIFIED;

    unsafe {
        let desc = &mut *desc;

        loop {
            let holder = ensure_holder(cx, wrapper);

            // Ordering is important here.
            //
            // We first need to call resolveOwnProperty, even before checking the
            // holder, because there might be a new dynamic |own| property that
            // appears and shadows a previously-resolved non-own property that we
            // cached on the holder. This can happen with indexed properties on
            // NodeLists, for example, which are |own| value props.
            //
            // resolveOwnProperty may or may not cache what it finds on the holder,
            // depending on how ephemeral it decides the property is. NodeList
            // |own| properties don't get defined on the holder, since they're
            // supposed to be dynamic. This means that we have to first check the
            // result of resolveOwnProperty, and _then_, if that comes up blank,
            // check the holder for any cached native properties.
            //
            // Finally, we call resolveNativeProperty, which checks non-own
            // properties, and unconditionally caches what it finds on the holder.

            // Check resolveOwnProperty.
            if !resolve_own_property(cx, wrapper, holder, id, flags, desc) {
                return false;
            }
            // Check the holder.
            if desc.obj.is_null() && JS_GetPropertyDescriptorById(cx, holder, id, flags, desc) == 0 {
                return false;
            }
            if desc.obj.is_not_null() {
                desc.obj = wrapper;
                break;
            }

            // Nothing in the cache. Call through, and cache the result.
            let obj = get_target_object(wrapper);
            if !XrayResolveNativeProperty(cx, wrapper, obj, id, desc) {
                return false;
            }

/*
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
                break;

            if (!JS_DefinePropertyById(cx, holder, id, desc.value(), desc.attributes(),
                                       desc.getter(), desc.setter()) ||
                !JS_GetPropertyDescriptorById(cx, holder, id, desc))
            {
                return false;
            }
            MOZ_ASSERT(desc.object());
            desc.object().set(wrapper);
            */
            break;
        }
        if desc.obj.is_not_null() {
            // All properties on cross-origin DOM objects are |own|.
            assert!(desc.obj == wrapper);

            // All properties on cross-origin DOM objects are non-enumerable and
            // "configurable". Any value attributes are read-only.
            desc.attrs &= !JSPROP_ENUMERATE;
            desc.attrs &= !JSPROP_PERMANENT;
            if desc.getter.is_none() && desc.setter.is_none() {
                desc.attrs |= JSPROP_READONLY;
            }
        }
        return true;
    }
}

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
    getPropertyDescriptor: Some(get_property_descriptor),
    getOwnPropertyDescriptor: Some(get_property_descriptor),
    defineProperty: Some(define_property),
    getOwnPropertyNames: 0 as *const u8,
    delete_: Some(delete),
    enumerate: 0 as *const u8,

    has: None, // -> BaseProxyHandler
    hasOwn: None, // -> BaseProxyHandler
    get: None,
    set: None,
    keys: 0 as *const u8, // -> BaseProxyHandler
    iterate: None, // -> BaseProxyHandler

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
