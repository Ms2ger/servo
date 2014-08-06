/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Various utilities to glue JavaScript and the DOM implementation together.

use dom::bindings::codegen::Bindings::WindowBinding;
use dom::bindings::codegen::PrototypeList;
use dom::bindings::codegen::PrototypeList::MAX_PROTO_CHAIN_LENGTH;
use dom::bindings::conversions::{FromJSValConvertible, IDLInterface};
use dom::bindings::global::{GlobalRef, GlobalField, WindowField, WorkerField};
use dom::bindings::js::{JS, Temporary, Root};
use dom::bindings::trace::Untraceable;
use dom::browsercontext;
use dom::window;
use servo_util::str::DOMString;

use libc;
use libc::c_uint;
use std::cell::Cell;
use std::mem;
use std::cmp::PartialEq;
use std::ptr;
use std::slice;
//use js::glue::{js_IsObjectProxyClass, js_IsFunctionProxyClass, IsProxyHandlerFamily};
//use js::glue::{GetGlobalForObjectCrossCompartment, UnwrapObject, GetProxyHandlerExtra};
use js::glue::{CompartmentOptions_SetVersion, CompartmentOptions_SetTraceGlobal};
use js::glue::{GetGlobalForObjectCrossCompartment, UnwrapObject};
use js::glue::{IsWrapper, RUST_JSID_TO_STRING, RUST_JSID_IS_INT};
use js::glue::{RUST_JSID_IS_STRING, RUST_JSID_TO_INT, ToString, NewGlobalObject};
use js::jsapi::{JS_AlreadyHasOwnProperty, JS_NewFunction, JSVERSION_LATEST};
use js::jsapi::{JS_DefineProperties, JS_ForwardGetPropertyTo, JSHandleValue};
use js::jsapi::{JS_GetClass, JS_LinkConstructorAndPrototype, JS_GetStringCharsAndLength};
use js::jsapi::{JS_ObjectIsRegExp, JS_ObjectIsDate, JSHandleObject, JSMutableHandleValue};
use js::jsapi::{JS_GetFunctionObject, JSMutableHandleObject, MutableHandle};
use js::jsapi::{JS_HasPropertyById, JS_GetPrototype};
use js::jsapi::{JS_GetProperty, JS_HasProperty};
use js::jsapi::{JS_DefineFunctions, JS_DefineProperty};
use js::jsapi::{JS_GetReservedSlot, JS_SetReservedSlot};
use js::jsapi::{JSContext, JSObject, jsid, JSClass};
use js::jsapi::{JSFunctionSpec, JSPropertySpec, JSHandleId};
use js::jsapi::{JS_InitStandardClasses};
use js::jsapi::{JSString, Handle};
use js::jsfriendapi::bindgen::JS_NewObjectWithUniqueType;
use js::jsval::JSVal;
use js::jsval::{PrivateValue, ObjectValue, NullValue, ObjectOrNullValue};
use js::jsval::{Int32Value, UInt32Value, DoubleValue, BooleanValue, UndefinedValue};
use js::rust::with_compartment;
use js::{JSPROP_ENUMERATE, JSCLASS_IS_GLOBAL, JSCLASS_IS_DOMJSCLASS};
use js::JSPROP_PERMANENT;
use js::{JSFUN_CONSTRUCTOR, JSPROP_READONLY};
use js;

pub type RustPropertyOp = Option<unsafe extern "C" fn
                                 (cx: *mut JSContext, argc: libc::c_uint, vp: *mut JSVal)
                                 -> bool>;
pub type RustStrictPropertyOp = Option<unsafe extern "C" fn
                                       (cx: *mut JSContext, argc: libc::c_uint, vp: *mut JSVal)
                                       -> bool>;

#[allow(raw_pointer_deriving)]
#[deriving(Encodable)]
pub struct GlobalStaticData {
    pub windowproxy_handler: Untraceable<*const libc::c_void>,
}

pub fn GlobalStaticData() -> GlobalStaticData {
    GlobalStaticData {
        windowproxy_handler: Untraceable::new(browsercontext::new_window_proxy_handler()),
    }
}

/// Returns whether the given `clasp` is one for a DOM object.
fn is_dom_class(clasp: *const JSClass) -> bool {
    unsafe {
        ((*clasp).flags & js::JSCLASS_IS_DOMJSCLASS) != 0
    }
}

pub unsafe fn dom_object_slot(obj: *mut JSObject) -> u32 {
    let clasp = JS_GetClass(obj);
    assert!(is_dom_class(clasp));
    DOM_OBJECT_SLOT as u32
}

/// Get the DOM object from the given reflector.
pub unsafe fn unwrap<T>(obj: *mut JSObject) -> *const T {
    let slot = dom_object_slot(obj);
    let val = JS_GetReservedSlot(obj, slot);
    val.to_private() as *const T
}

/// Get the `DOMClass` from `obj`, or `Err(())` if `obj` is not a DOM object.
pub unsafe fn get_dom_class(obj: *mut JSObject) -> Result<DOMClass, ()> {
    let clasp = JS_GetClass(obj);
    if is_dom_class(&*clasp) {
        debug!("plain old dom object");
        let domjsclass: *const DOMJSClass = clasp as *const DOMJSClass;
        return Ok((*domjsclass).dom_class);
    }
    debug!("not a dom object");
    return Err(());
}

/// Get a `JS<T>` for the given DOM object, unwrapping any wrapper around it
/// first, and checking if the object is of the correct type.
///
/// Returns Err(()) if `obj` is an opaque security wrapper or if the object is
/// not a reflector for a DOM object of the given type (as defined by the
/// proto_id and proto_depth).
pub fn unwrap_jsmanaged<T: Reflectable>(mut obj: *mut JSObject,
                                        proto_id: PrototypeList::id::ID,
                                        proto_depth: uint) -> Result<JS<T>, ()> {
    unsafe {
        let dom_class = get_dom_class(obj).or_else(|_| {
            if IsWrapper(obj) == 1 {
                debug!("found wrapper");
                obj = UnwrapObject(obj, /* stopAtOuter = */ false);
                if obj.is_null() {
                    debug!("unwrapping security wrapper failed");
                    Err(())
                } else {
                    assert!(IsWrapper(obj) == 0);
                    debug!("unwrapped successfully");
                    get_dom_class(obj)
                }
            } else {
                debug!("not a dom wrapper");
                Err(())
            }
        });

        dom_class.and_then(|dom_class| {
            if dom_class.interface_chain[proto_depth] == proto_id {
                debug!("good prototype");
                Ok(JS::from_raw(unwrap(obj)))
            } else {
                debug!("bad prototype");
                Err(())
            }
        })
    }
}

/// Leak the given pointer.
pub unsafe fn squirrel_away_unique<T>(x: Box<T>) -> *const T {
    mem::transmute(x)
}

/// Convert the given `JSString` to a `DOMString`. Fails if the string does not
/// contain valid UTF-16.
pub fn jsstring_to_str(cx: *mut JSContext, s: *mut JSString) -> DOMString {
    unsafe {
        let mut length = 0;
        let chars = JS_GetStringCharsAndLength(cx, s, &mut length);
        slice::raw::buf_as_slice(chars, length as uint, |char_vec| {
            String::from_utf16(char_vec).unwrap()
        })
    }
}

/// Convert the given `jsid` to a `DOMString`. Fails if the `jsid` is not a
/// string, or if the string does not contain valid UTF-16.
pub fn jsid_to_str(cx: *mut JSContext, id: jsid) -> DOMString {
    unsafe {
        assert!(RUST_JSID_IS_STRING(id));
        jsstring_to_str(cx, RUST_JSID_TO_STRING(id))
    }
}

/// The index of the slot wherein a pointer to the reflected DOM object is
/// stored for non-proxy bindings.
// We use slot 0 for holding the raw object.  This is safe for both
// globals and non-globals.
pub static DOM_OBJECT_SLOT: uint = 0;

#[static_assert]
static DOM_SLOT_IS_PROXY_PRIVATE: bool = DOM_OBJECT_SLOT == js::JSSLOT_PROXY_PRIVATE as uint;

// NOTE: This is baked into the Ion JIT as 0 in codegen for LGetDOMProperty and
// LSetDOMProperty. Those constants need to be changed accordingly if this value
// changes.
static DOM_PROTO_INSTANCE_CLASS_SLOT: u32 = 0;

/// The index of the slot that contains a reference to the ProtoOrIfaceArray.
// All DOM globals must have a slot at DOM_PROTOTYPE_SLOT.
pub static DOM_PROTOTYPE_SLOT: u32 = js::JSCLASS_GLOBAL_SLOT_COUNT;

/// The flag set on the `JSClass`es for DOM global objects.
// NOTE: This is baked into the Ion JIT as 0 in codegen for LGetDOMProperty and
// LSetDOMProperty. Those constants need to be changed accordingly if this value
// changes.
pub static JSCLASS_DOM_GLOBAL: u32 = js::JSCLASS_USERBIT1;

/// Representation of an IDL constant value.
#[deriving(Clone)]
pub enum ConstantVal {
    IntVal(i32),
    UintVal(u32),
    DoubleVal(f64),
    BoolVal(bool),
    NullVal,
    VoidVal
}

/// Representation of an IDL constant.
#[deriving(Clone)]
pub struct ConstantSpec {
    pub name: &'static [u8],
    pub value: ConstantVal
}

/// The struct that holds inheritance information for DOM object reflectors.
pub struct DOMClass {
    /// A list of interfaces that this object implements, in order of decreasing
    /// derivedness.
    pub interface_chain: [PrototypeList::id::ID, ..MAX_PROTO_CHAIN_LENGTH]
}

/// The JSClass used for DOM object reflectors.
pub struct DOMJSClass {
    pub base: js::Class,
    pub dom_class: DOMClass
}

/// Returns the ProtoOrIfaceArray for the given global object.
/// Fails if `global` is not a DOM global object.
pub fn GetProtoOrIfaceArray(global: *mut JSObject) -> *mut *mut JSObject {
    unsafe {
        assert!(((*JS_GetClass(global)).flags & JSCLASS_DOM_GLOBAL) != 0);
        JS_GetReservedSlot(global, DOM_PROTOTYPE_SLOT).to_private() as *mut *mut JSObject
    }
}

/// Contains references to lists of methods, attributes, and constants for a
/// given interface.
pub struct NativeProperties {
    pub methods: Option<&'static [JSFunctionSpec]>,
    pub attrs: Option<&'static [JSPropertySpec]>,
    pub consts: Option<&'static [ConstantSpec]>,
    pub staticMethods: Option<&'static [JSFunctionSpec]>,
    pub staticAttrs: Option<&'static [JSPropertySpec]>,
}

/// A JSNative that cannot be null.
pub type NonNullJSNative =
    unsafe extern "C" fn (arg1: *mut JSContext, arg2: c_uint, arg3: *mut JSVal) -> bool;

/// Creates the *interface prototype object* and the *interface object* (if
/// needed).
/// Fails on JSAPI failure.
pub fn CreateInterfaceObjects2(cx: *mut JSContext, global: JSHandleObject, receiver: *mut JSObject,
                               protoProto: JSHandleObject,
                               protoClass: &'static JSClass,
                               constructor: Option<(NonNullJSNative, &'static str, u32)>,
                               domClass: *const DOMClass,
                               members: &'static NativeProperties) -> *mut JSObject {
    let proto = CreateInterfacePrototypeObject(cx, global, protoProto,
                                               protoClass, members);

    unsafe {
        JS_SetReservedSlot(proto, DOM_PROTO_INSTANCE_CLASS_SLOT,
                           PrivateValue(domClass as *const libc::c_void));
    }

    match constructor {
        Some((native, name, nargs)) => {
            let s = name.to_c_str();
            CreateInterfaceObject(cx, global, receiver,
                                  native, nargs, proto,
                                  members, s.as_ptr())
        },
        None => (),
    }

    proto
}

/// Creates the *interface object*.
/// Fails on JSAPI failure.
fn CreateInterfaceObject(cx: *mut JSContext, global: JSHandleObject, receiver: *mut JSObject,
                         constructorNative: NonNullJSNative,
                         ctorNargs: u32, proto: *mut JSObject,
                         members: &'static NativeProperties,
                         name: *const libc::c_char) {
    unsafe {
        let fun = JS_NewFunction(cx, Some(constructorNative), ctorNargs,
                                 JSFUN_CONSTRUCTOR, global, name);
        assert!(fun.is_not_null());

        let constructor = JS_GetFunctionObject(fun);
        assert!(constructor.is_not_null());

        let constructor = object_handle(&constructor);
        match members.staticMethods {
            Some(staticMethods) => DefineMethods(cx, constructor, staticMethods),
            _ => (),
        }

        match members.staticAttrs {
            Some(staticProperties) => DefineProperties(cx, constructor, staticProperties),
            _ => (),
        }

        match members.consts {
            Some(constants) => DefineConstants(cx, constructor, constants),
            _ => (),
        }

        let protohandle = object_handle(&proto);
        if proto.is_not_null() {
            assert!(JS_LinkConstructorAndPrototype(cx, constructor, protohandle));
        }

        let mut alreadyDefined = false;
        let receiverhandle = object_handle(&receiver);
        assert!(JS_AlreadyHasOwnProperty(cx, receiverhandle, name, &mut alreadyDefined));

        let constructorhandle = ObjectValue(&**constructor);
        if !alreadyDefined {
            assert!(JS_DefineProperty(cx, receiverhandle, name,
                                      value_handle(&constructorhandle),
                                      0, None, None));
        }
    }
}

/// Defines constants on `obj`.
/// Fails on JSAPI failure.
fn DefineConstants(cx: *mut JSContext, obj: JSHandleObject, constants: &'static [ConstantSpec]) {
    for spec in constants.iter() {
        let jsval = match spec.value {
            NullVal => NullValue(),
            IntVal(i) => Int32Value(i),
            UintVal(u) => UInt32Value(u),
            DoubleVal(d) => DoubleValue(d),
            BoolVal(b) => BooleanValue(b),
            VoidVal => UndefinedValue(),
        };
        let jsval = value_handle(&jsval);
        unsafe {
            assert!(JS_DefineProperty(cx, obj, spec.name.as_ptr() as *const libc::c_char, jsval,
                                      JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT,
                                      None, None));
        }
    }
}

/// Defines methods on `obj`. The last entry of `methods` must contain zeroed
/// memory.
/// Fails on JSAPI failure.
fn DefineMethods(cx: *mut JSContext, obj: JSHandleObject, methods: &'static [JSFunctionSpec]) {
    unsafe {
        assert!(JS_DefineFunctions(cx, obj, methods.as_ptr()));
    }
}

/// Defines attributes on `obj`. The last entry of `properties` must contain
/// zeroed memory.
/// Fails on JSAPI failure.
fn DefineProperties(cx: *mut JSContext, obj: JSHandleObject, properties: &'static [JSPropertySpec]) {
    unsafe {
        assert!(JS_DefineProperties(cx, obj, properties.as_ptr()));
    }
}

/// Creates the *interface prototype object*.
/// Fails on JSAPI failure.
fn CreateInterfacePrototypeObject(cx: *mut JSContext, global: JSHandleObject,
                                  parentProto: JSHandleObject,
                                  protoClass: &'static JSClass,
                                  members: &'static NativeProperties) -> *mut JSObject {
    unsafe {
        let ourProto = JS_NewObjectWithUniqueType(cx, protoClass, parentProto, global);
        assert!(ourProto.is_not_null());

        let ourProto = object_handle(&ourProto);
        match members.methods {
            Some(methods) => DefineMethods(cx, ourProto, methods),
            _ => (),
        }

        match members.attrs {
            Some(properties) => DefineProperties(cx, ourProto, properties),
            _ => (),
        }

        match members.consts {
            Some(constants) => DefineConstants(cx, ourProto, constants),
            _ => (),
        }

        return *ourProto.unnamed_field1;
    }
}

/// A throwing constructor, for those interfaces that have neither
/// `NoInterfaceObject` nor `Constructor`.
pub extern fn ThrowingConstructor(_cx: *mut JSContext, _argc: c_uint, _vp: *mut JSVal) -> bool {
    // FIXME(#347) should trigger exception here
    return false;
}

/// Construct and cache the ProtoOrIfaceArray for the given global.
/// Fails if the argument is not a DOM global.
pub fn initialize_global(global: *mut JSObject) {
    let protoArray = box () ([0 as *mut JSObject, ..PrototypeList::id::IDCount as uint]);
    unsafe {
        assert!(((*JS_GetClass(global)).flags & JSCLASS_DOM_GLOBAL) != 0);
        let box_ = squirrel_away_unique(protoArray);
        JS_SetReservedSlot(global,
                           DOM_PROTOTYPE_SLOT,
                           PrivateValue(box_ as *const libc::c_void));
    }
}

/// A trait to provide access to the `Reflector` for a DOM object.
pub trait Reflectable {
    fn reflector<'a>(&'a self) -> &'a Reflector;
}

/// Create the reflector for a new DOM object and yield ownership to the
/// reflector.
pub fn reflect_dom_object<T: Reflectable>
        (obj:     Box<T>,
         global:  &GlobalRef,
         wrap_fn: extern "Rust" fn(*mut JSContext, &GlobalRef, Box<T>) -> Temporary<T>)
         -> Temporary<T> {
    wrap_fn(global.get_cx(), global, obj)
}

/// A struct to store a reference to the reflector of a DOM object.
#[allow(raw_pointer_deriving)]
#[deriving(PartialEq)]
pub struct Reflector {
    object: Cell<*mut JSObject>,
}

impl Reflector {
    /// Get the reflector.
    #[inline]
    pub fn get_jsobject(&self) -> *mut JSObject {
        self.object.get()
    }

    /// Initialize the reflector. (May be called only once.)
    pub fn set_jsobject(&self, object: *mut JSObject) {
        assert!(self.object.get().is_null());
        assert!(object.is_not_null());
        self.object.set(object);
    }

    /// Return a pointer to the memory location at which the JS reflector object is stored.
    /// Used by Temporary values to root the reflector, as required by the JSAPI rooting
    /// APIs.
    pub fn rootable(&self) -> *mut *mut JSObject {
        &self.object as *const Cell<*mut JSObject>
                     as *mut Cell<*mut JSObject>
                     as *mut *mut JSObject
    }

    /// Create an uninitialized `Reflector`.
    pub fn new() -> Reflector {
        Reflector {
            object: Cell::new(ptr::mut_null()),
        }
    }
}

pub fn GetPropertyOnPrototype(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId, found: *mut bool,
                              vp: Option<JSMutableHandleValue>) -> bool {
    unsafe {
      //let proto = GetObjectProto(proxy);
      let mut proto = ptr::mut_null();
      if !JS_GetPrototype(cx, proxy, mut_object_handle(&mut proto)) {
          return false;
      }
      if proto.is_null() {
          *found = false;
          return true;
      }
      if !JS_HasPropertyById(cx, object_handle(&proto), id, found) {
          return false;
      }
      let no_output = vp.as_ref().map_or(true, |vp| vp.deref().is_null());
      if !*found || no_output {
          return true;
      }

      JS_ForwardGetPropertyTo(cx, object_handle(&proto), id, proxy, vp.unwrap())
  }
}

/// Get an array index from the given `jsid`. Returns `None` if the given
/// `jsid` is not an integer.
pub fn GetArrayIndexFromId(_cx: *mut JSContext, id: jsid) -> Option<u32> {
    unsafe {
        if RUST_JSID_IS_INT(id) {
            return Some(RUST_JSID_TO_INT(id) as u32);
        }
        return None;
    }
    // if id is length atom, -1, otherwise
    /*return if JSID_IS_ATOM(id) {
        let atom = JSID_TO_ATOM(id);
        //let s = *GetAtomChars(id);
        if s > 'a' && s < 'z' {
            return -1;
        }

        let i = 0;
        let str = AtomToLinearString(JSID_TO_ATOM(id));
        return if StringIsArray(str, &mut i) != 0 { i } else { -1 }
    } else {
        IdToInt32(cx, id);
    }*/
}

/// Find the index of a string given by `v` in `values`.
/// Returns `Err(())` on JSAPI failure (there is a pending exception), and
/// `Ok(None)` if there was no matching string.
pub fn FindEnumStringIndex(cx: *mut JSContext,
                           v: JSVal,
                           values: &[&'static str]) -> Result<Option<uint>, ()> {
    unsafe {
        let v = value_handle(&v);
        let jsstr = ToString(cx, v);
        if jsstr.is_null() {
            return Err(());
        }

        let mut length = 0;
        let chars = JS_GetStringCharsAndLength(cx, jsstr, &mut length);
        if chars.is_null() {
            return Err(());
        }

        Ok(values.iter().position(|value| {
            value.len() == length as uint &&
            range(0, length as uint).all(|j| {
                value.as_bytes()[j] as u16 == *chars.offset(j as int)
            })
        }))
    }
}

/// Get the property with name `property` from `object`.
/// Returns `Err(())` on JSAPI failure (there is a pending exception), and
/// `Ok(None)` if there was no property with the given name.
pub fn get_dictionary_property(cx: *mut JSContext,
                               object: JSHandleObject,
                               property: &str) -> Result<Option<JSVal>, ()> {
    use std::c_str::CString;
    fn has_property(cx: *mut JSContext, object: JSHandleObject,
                    property: &CString) -> Result<bool, ()> {
        let mut found = false;
        if unsafe { JS_HasProperty(cx, object, property.as_ptr(), &mut found) } {
            Ok(found)
        } else {
            Err(())
        }
    }
    /*fn get_property(cx: *mut JSContext, object: JSHandleObject, property: &CString,
                    value: JSMutableHandleValue) -> bool {
        unsafe {
            JS_GetProperty(cx, object, property.as_ptr(), value) != 0
        }
    }*/

    let property = property.to_c_str();
    if unsafe { (*object.unnamed_field1).is_null() } {
        return Ok(None);
    }

    let found = try!(has_property(cx, object, &property));
    if !found {
        return Ok(None);
    }

    let mut value = NullValue();
    /*if !get_property(cx, object, &property, mut_value_handle(&mut value)) {
        return Err(());
    }*/
    if unsafe {
        property.with_ref(|s| {
            !JS_GetProperty(cx, object, s, mut_value_handle(&mut value))
        })
    } {
        return Err(());
    }

    unsafe {
        Ok(Some(value))
    }
}

pub fn HasPropertyOnPrototype(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId) -> bool {
    //  MOZ_ASSERT(js::IsProxy(proxy) && js::GetProxyHandler(proxy) == handler);
    let mut found = false;
    return !GetPropertyOnPrototype(cx, proxy, id, &mut found, None) || found;
}

/// Returns whether `obj` can be converted to a callback interface per IDL.
pub fn IsConvertibleToCallbackInterface(cx: *mut JSContext, obj: *mut JSObject) -> bool {
    unsafe {
        let obj = object_handle(&obj);
        !JS_ObjectIsDate(cx, obj) && !JS_ObjectIsRegExp(cx, obj)
    }
}

/// Create a DOM global object with the given class.
pub fn CreateDOMGlobal(cx: *mut JSContext, class: *const JSClass) -> *mut JSObject {
    unsafe {
        //XXXjdm need to trace the protoiface cache, too
        let obj = NewGlobalObject(cx, class, ptr::mut_null(), 0 /*FireOnNewGlobalHook*/);
        if obj.is_null() {
            return ptr::mut_null();
        }
        with_compartment(cx, obj, || {
            let globhandle = object_handle(&obj);
            CompartmentOptions_SetVersion(cx, JSVERSION_LATEST);
            CompartmentOptions_SetTraceGlobal(cx, Some(WindowBinding::_trace));
            JS_InitStandardClasses(cx, globhandle);
        });
        initialize_global(obj);
        obj
    }
}

/*
/// Callback to outerize windows when wrapping.
pub extern fn wrap_for_same_compartment(cx: *mut JSContext, obj: *mut JSObject) -> *mut JSObject {
    unsafe {
        JS_ObjectToOuterObject(cx, obj)
    }
}

/// Callback to outerize windows before wrapping.
pub extern fn pre_wrap(cx: *mut JSContext, _scope: *mut JSObject,
                       obj: *mut JSObject, _flags: c_uint) -> *mut JSObject {
    unsafe {
        JS_ObjectToOuterObject(cx, obj)
    }
}
*/

/// Callback to outerize windows.
pub extern fn outerize_global(_cx: *mut JSContext, obj: JSHandleObject) -> *mut JSObject {
    unsafe {
        debug!("outerizing");
        let obj = *obj.unnamed_field1;
        let win: Root<window::Window> =
            unwrap_jsmanaged(obj,
                             IDLInterface::get_prototype_id(None::<window::Window>),
                             IDLInterface::get_prototype_depth(None::<window::Window>))
            .unwrap()
            .root();
        win.deref().browser_context.deref().borrow().get_ref().window_proxy()
    }
}

/// Returns the global object of the realm that the given JS object was created in.
pub fn global_object_for_js_object(obj: *mut JSObject) -> GlobalField {
    unsafe {
        let global = GetGlobalForObjectCrossCompartment(obj);
        let clasp = JS_GetClass(global);
        assert!(((*clasp).flags & (JSCLASS_IS_DOMJSCLASS | JSCLASS_IS_GLOBAL)) != 0);
        match FromJSValConvertible::from_jsval(ptr::mut_null(), ObjectOrNullValue(global), ()) {
            Ok(window) => return WindowField(window),
            Err(_) => (),
        }

        match FromJSValConvertible::from_jsval(ptr::mut_null(), ObjectOrNullValue(global), ()) {
            Ok(worker) => return WorkerField(worker),
            Err(_) => (),
        }

        fail!("found DOM global that doesn't unwrap to Window or WorkerGlobalScope")
    }
}

/// Get the `JSContext` for the `JSRuntime` associated with the thread
/// this object is on.
fn cx_for_dom_reflector(obj: *mut JSObject) -> *mut JSContext {
    let global = global_object_for_js_object(obj).root();
    global.root_ref().get_cx()
}

/// Get the `JSContext` for the `JSRuntime` associated with the thread
/// this DOM object is on.
pub fn cx_for_dom_object<T: Reflectable>(obj: &T) -> *mut JSContext {
    cx_for_dom_reflector(obj.reflector().get_jsobject())
}

/// Results of `xml_name_type`.
#[deriving(PartialEq)]
pub enum XMLName {
    QName,
    Name,
    InvalidXMLName
}

/// Check if an element name is valid. See http://www.w3.org/TR/xml/#NT-Name
/// for details.
pub fn xml_name_type(name: &str) -> XMLName {
    fn is_valid_start(c: char) -> bool {
        match c {
            ':' |
            'A' .. 'Z' |
            '_' |
            'a' .. 'z' |
            '\xC0' .. '\xD6' |
            '\xD8' .. '\xF6' |
            '\xF8' .. '\u02FF' |
            '\u0370' .. '\u037D' |
            '\u037F' .. '\u1FFF' |
            '\u200C' .. '\u200D' |
            '\u2070' .. '\u218F' |
            '\u2C00' .. '\u2FEF' |
            '\u3001' .. '\uD7FF' |
            '\uF900' .. '\uFDCF' |
            '\uFDF0' .. '\uFFFD' |
            '\U00010000' .. '\U000EFFFF' => true,
            _ => false,
        }
    }

    fn is_valid_continuation(c: char) -> bool {
        is_valid_start(c) || match c {
            '-' |
            '.' |
            '0' .. '9' |
            '\xB7' |
            '\u0300' .. '\u036F' |
            '\u203F' .. '\u2040' => true,
            _ => false,
        }
    }

    let mut iter = name.chars();
    let mut non_qname_colons = false;
    let mut seen_colon = false;
    match iter.next() {
        None => return InvalidXMLName,
        Some(c) => {
            if !is_valid_start(c) {
                return InvalidXMLName;
            }
            if c == ':' {
                non_qname_colons = true;
            }
        }
    }

    for c in name.chars() {
        if !is_valid_continuation(c) {
            return InvalidXMLName;
        }
        if c == ':' {
            match seen_colon {
                true => non_qname_colons = true,
                false => seen_colon = true
            }
        }
    }

    match non_qname_colons {
        false => QName,
        true => Name
    }
}

pub fn object_handle<'a>(obj: &'a *mut JSObject) -> JSHandleObject<'a> {
    Handle {
        unnamed_field1: obj
    }
}

pub fn mut_object_handle<'a>(obj: &'a mut *mut JSObject) -> JSMutableHandleObject<'a> {
    MutableHandle {
        unnamed_field1: obj
    }
}

pub fn id_handle<'a>(id: &'a jsid) -> JSHandleId<'a> {
    Handle {
        unnamed_field1: id
    }
}

pub fn value_handle<'a>(val: &'a JSVal) -> JSHandleValue<'a> {
    Handle {
        unnamed_field1: val
    }
}

pub fn mut_value_handle<'a>(val: &'a mut JSVal) -> JSMutableHandleValue<'a> {
    MutableHandle {
        unnamed_field1: val
    }
}

pub fn mut_handle<'a, T>(val: &'a mut T) -> MutableHandle<'a, T> {
    MutableHandle {
        unnamed_field1: val
    }
}
