/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::PrototypeList;
use dom::bindings::codegen::PrototypeList::MAX_PROTO_CHAIN_LENGTH;
use dom::bindings::conversions::{FromJSValConvertible, IDLInterface};
use dom::bindings::js::{JS, JSRef, Temporary, Root};
use dom::bindings::trace::Untraceable;
use dom::browsercontext;
use dom::window;
use servo_util::str::DOMString;

use collections::hashmap::HashMap;
use libc;
use libc::c_uint;
use std::mem;
use std::cmp::Eq;
use std::ptr;
use std::slice;
use std::str;
use js::glue::{GetGlobalForObjectCrossCompartment, UnwrapObject};
use js::glue::{IsWrapper, RUST_JSID_TO_STRING, RUST_JSID_IS_INT};
use js::glue::{RUST_JSID_IS_STRING, RUST_JSID_TO_INT, ToString, NewGlobalObject};
use js::jsapi::{JS_AlreadyHasOwnProperty, JS_NewFunction};
use js::jsapi::{JS_DefineProperties, JS_ForwardGetPropertyTo, JSHandleValue};
use js::jsapi::{JS_GetClass, JS_LinkConstructorAndPrototype, JS_GetStringCharsAndLength};
use js::jsapi::{JS_ObjectIsRegExp, JS_ObjectIsDate, JSHandleObject, JSMutableHandleValue};
use js::jsapi::{JS_GetFunctionObject, JSMutableHandleObject};
use js::jsapi::{JS_HasPropertyById, JS_GetPrototype};
use js::jsapi::{JS_GetProperty, JS_HasProperty};
use js::jsapi::{JS_DefineFunctions, JS_DefineProperty};
use js::jsapi::{JS_GetReservedSlot, JS_SetReservedSlot};
use js::jsapi::{JSContext, JSObject, JSBool, jsid, JSClass, JSNative};
use js::jsapi::{JSFunctionSpec, JSPropertySpec, JSHandleId};
use js::jsapi::{JS_InitStandardClasses};
use js::jsapi::{JSString, Handle, MutableHandle};
use js::jsfriendapi::JS_ObjectToOuterObject;
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
                                 -> JSBool>;
pub type RustStrictPropertyOp = Option<unsafe extern "C" fn
                                       (cx: *mut JSContext, argc: libc::c_uint, vp: *mut JSVal)
                                       -> JSBool>;

#[allow(raw_pointer_deriving)]
#[deriving(Encodable)]
pub struct GlobalStaticData {
    pub proxy_handlers: Untraceable<HashMap<uint, *libc::c_void>>,
    pub windowproxy_handler: Untraceable<*libc::c_void>,
}

pub fn GlobalStaticData() -> GlobalStaticData {
    GlobalStaticData {
        proxy_handlers: Untraceable::new(HashMap::new()),
        windowproxy_handler: Untraceable::new(browsercontext::new_window_proxy_handler()),
    }
}

fn is_dom_class(clasp: *JSClass) -> bool {
    unsafe {
        ((*clasp).flags & js::JSCLASS_IS_DOMJSCLASS) != 0
    }
}

pub unsafe fn dom_object_slot(obj: *mut JSObject) -> u32 {
    let clasp = JS_GetClass(obj);
    assert!(is_dom_class(clasp));
    DOM_OBJECT_SLOT as u32
}

pub unsafe fn unwrap<T>(obj: *mut JSObject) -> *T {
    let slot = dom_object_slot(obj);
    let val = JS_GetReservedSlot(obj, slot);
    val.to_private() as *T
}

pub unsafe fn get_dom_class(obj: *mut JSObject) -> Result<DOMClass, ()> {
    let clasp = JS_GetClass(obj);
    if is_dom_class(clasp) {
        debug!("plain old dom object");
        let domjsclass: *DOMJSClass = clasp as *DOMJSClass;
        return Ok((*domjsclass).dom_class);
    }
    debug!("not a dom object");
    return Err(());
}

pub fn unwrap_jsmanaged<T: Reflectable>(mut obj: *mut JSObject,
                                        proto_id: PrototypeList::id::ID,
                                        proto_depth: uint) -> Result<JS<T>, ()> {
    unsafe {
        let dom_class = get_dom_class(obj).or_else(|_| {
            if IsWrapper(obj) == 1 {
                debug!("found wrapper");
                obj = UnwrapObject(obj, /* stopAtOuter = */ 0);
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

pub unsafe fn squirrel_away_unique<T>(x: Box<T>) -> *T {
    mem::transmute(x)
}

pub fn jsstring_to_str(cx: *mut JSContext, s: *mut JSString) -> DOMString {
    unsafe {
        let mut length = 0;
        let chars = JS_GetStringCharsAndLength(cx, s, &mut length);
        slice::raw::buf_as_slice(chars, length as uint, |char_vec| {
            str::from_utf16(char_vec).unwrap()
        })
    }
}

pub fn jsid_to_str(cx: *mut JSContext, id: JSHandleId) -> DOMString {
    unsafe {
        assert!(RUST_JSID_IS_STRING(*id) != 0);
        jsstring_to_str(cx, RUST_JSID_TO_STRING(*id))
    }
}

// We use slot 0 for holding the raw object.  This is safe for both
// globals and non-globals.
pub static DOM_OBJECT_SLOT: uint = 0;

#[static_assert]
static DOM_SLOT_IS_PROXY_PRIVATE: bool = DOM_OBJECT_SLOT == js::JSSLOT_PROXY_PRIVATE as uint;

// NOTE: This is baked into the Ion JIT as 0 in codegen for LGetDOMProperty and
// LSetDOMProperty. Those constants need to be changed accordingly if this value
// changes.
static DOM_PROTO_INSTANCE_CLASS_SLOT: u32 = 0;

// All DOM globals must have a slot at DOM_PROTOTYPE_SLOT.
pub static DOM_PROTOTYPE_SLOT: u32 = js::JSCLASS_GLOBAL_SLOT_COUNT;

// NOTE: This is baked into the Ion JIT as 0 in codegen for LGetDOMProperty and
// LSetDOMProperty. Those constants need to be changed accordingly if this value
// changes.
pub static JSCLASS_DOM_GLOBAL: u32 = js::JSCLASS_USERBIT1;

#[deriving(Clone)]
pub enum ConstantVal {
    IntVal(i32),
    UintVal(u32),
    DoubleVal(f64),
    BoolVal(bool),
    NullVal,
    VoidVal
}

#[deriving(Clone)]
pub struct ConstantSpec {
    pub name: &'static [u8],
    pub value: ConstantVal
}

pub struct DOMClass {
    // A list of interfaces that this object implements, in order of decreasing
    // derivedness.
    pub interface_chain: [PrototypeList::id::ID, ..MAX_PROTO_CHAIN_LENGTH]
}

pub struct DOMJSClass {
    pub base: js::Class,
    pub dom_class: DOMClass
}

pub fn GetProtoOrIfaceArray(global: *mut JSObject) -> *mut *mut JSObject {
    unsafe {
        assert!(((*JS_GetClass(global)).flags & JSCLASS_DOM_GLOBAL) != 0);
        JS_GetReservedSlot(global, DOM_PROTOTYPE_SLOT).to_private() as *mut *mut JSObject
    }
}

pub fn CreateInterfaceObjects2(cx: *mut JSContext, global: JSHandleObject, receiver: *mut JSObject,
                               protoProto: JSHandleObject, protoClass: *JSClass,
                               constructor: JSNative,
                               ctorNargs: u32,
                               domClass: *DOMClass,
                               methods: Option<&'static [JSFunctionSpec]>,
                               properties: Option<&'static [JSPropertySpec]>,
                               constants: Option<&'static [ConstantSpec]>,
                               staticMethods: Option<&'static [JSFunctionSpec]>,
                               name: &str) -> *mut JSObject {
    let mut proto = ptr::mut_null();
    if protoClass.is_not_null() {
        proto = CreateInterfacePrototypeObject(cx, global, protoProto,
                                               protoClass, methods,
                                               properties, constants);
        if proto.is_null() {
            return ptr::mut_null();
        }

        unsafe {
            JS_SetReservedSlot(proto, DOM_PROTO_INSTANCE_CLASS_SLOT,
                               PrivateValue(domClass as *libc::c_void));
        }
    }

    let mut interface = ptr::mut_null();
    if constructor.is_some() {
        interface = name.to_c_str().with_ref(|s| {
            CreateInterfaceObject(cx, unsafe { *global.unnamed_field1 }, receiver,
                                  constructor, ctorNargs, proto,
                                  staticMethods, constants, s)
        });
        if interface.is_null() {
            return ptr::mut_null();
        }
    }

    if protoClass.is_not_null() {
        proto
    } else {
        interface
    }
}

fn CreateInterfaceObject(cx: *mut JSContext, global: *mut JSObject, receiver: *mut JSObject,
                         constructorNative: JSNative,
                         ctorNargs: u32, proto: *mut JSObject,
                         staticMethods: Option<&'static [JSFunctionSpec]>,
                         constants: Option<&'static [ConstantSpec]>,
                         name: *libc::c_char) -> *mut JSObject {
    unsafe {
        let globalhandle = Handle {
            unnamed_field1: &global,
        };
        let fun = JS_NewFunction(cx, constructorNative, ctorNargs,
                                 JSFUN_CONSTRUCTOR, globalhandle, name);
        if fun.is_null() {
            return ptr::mut_null();
        }

        let constructor = JS_GetFunctionObject(fun);
        assert!(constructor.is_not_null());

        let constructor = Handle {
            unnamed_field1: &constructor,
        };
        match staticMethods {
            Some(staticMethods) => {
                if !DefineMethods(cx, constructor, staticMethods) {
                    return ptr::mut_null();
                }
            },
            _ => (),
        }

        match constants {
            Some(constants) => {
                if !DefineConstants(cx, constructor, constants) {
                    return ptr::mut_null();
                }
            },
            _ => (),
        }

        let protohandle = Handle {
            unnamed_field1: &proto,
        };
        if proto.is_not_null() && JS_LinkConstructorAndPrototype(cx, constructor, protohandle) == 0 {
            return ptr::mut_null();
        }

        let mut alreadyDefined = 0;
        let receiver = Handle {
            unnamed_field1: &receiver,
        };
        if JS_AlreadyHasOwnProperty(cx, receiver, name, &mut alreadyDefined) == 0 {
            return ptr::mut_null();
        }

        let constructorhandle = Handle {
            unnamed_field1: &ObjectValue(&**constructor.unnamed_field1),
        };
        if alreadyDefined == 0 &&
            JS_DefineProperty(cx, receiver, name, constructorhandle, 0, None, None) == 0 {
            return ptr::mut_null();
        }

        return *constructor.unnamed_field1;
    }
}

fn DefineConstants(cx: *mut JSContext, obj: *mut JSObject, constants: &'static [ConstantSpec]) -> bool {
    constants.iter().all(|spec| {
        let jsval = match spec.value {
            NullVal => NullValue(),
            IntVal(i) => Int32Value(i),
            UintVal(u) => UInt32Value(u),
            DoubleVal(d) => DoubleValue(d),
            BoolVal(b) => BooleanValue(b),
            VoidVal => UndefinedValue(),
        };
        let jsval = Handle {
            unnamed_field1: &jsval,
        };
        unsafe {
            JS_DefineProperty(cx, obj, spec.name, jsval,
                              JSPROP_ENUMERATE | JSPROP_READONLY | JSPROP_PERMANENT,
                              None, None)
        }
    })
}

fn DefineMethods(cx: *mut JSContext, obj: JSHandleObject, methods: &'static [JSFunctionSpec]) -> bool {
    unsafe {
        JS_DefineFunctions(cx, obj, methods.as_ptr()) != 0
    }
}

fn DefineProperties(cx: *mut JSContext, obj: JSHandleObject, properties: &'static [JSPropertySpec]) -> bool {
    unsafe {
        JS_DefineProperties(cx, obj, properties.as_ptr()) != 0
    }
}

fn CreateInterfacePrototypeObject(cx: *mut JSContext, global: JSHandleObject,
                                  parentProto: JSHandleObject, protoClass: *JSClass,
                                  methods: Option<&'static [JSFunctionSpec]>,
                                  properties: Option<&'static [JSPropertySpec]>,
                                  constants: Option<&'static [ConstantSpec]>) -> *mut JSObject {
    unsafe {
        let ourProto = JS_NewObjectWithUniqueType(cx, protoClass, parentProto, global);
        if ourProto.is_null() {
            return ptr::mut_null();
        }

        let ourProto = Handle {
            unnamed_field1: &ourProto,
        };

        match methods {
            Some(methods) => {
                if !DefineMethods(cx, ourProto, methods) {
                    return ptr::mut_null();
                }
            },
            _ => (),
        }

        match properties {
            Some(properties) => {
                if !DefineProperties(cx, ourProto, properties) {
                    return ptr::mut_null();
                }
            },
            _ => (),
        }

        match constants {
            Some(constants) => {
                if !DefineConstants(cx, ourProto, constants) {
                    return ptr::mut_null();
                }
            },
            _ => (),
        }

        return *ourProto.unnamed_field1;
    }
}

pub extern fn ThrowingConstructor(_cx: *mut JSContext, _argc: c_uint, _vp: *mut JSVal) -> JSBool {
    //XXX should trigger exception here
    return 0;
}

pub fn initialize_global(global: *mut JSObject) {
    let protoArray = box () ([0 as *mut JSObject, ..PrototypeList::id::IDCount as uint]);
    unsafe {
        let box_ = squirrel_away_unique(protoArray);
        JS_SetReservedSlot(global,
                           DOM_PROTOTYPE_SLOT,
                           PrivateValue(box_ as *libc::c_void));
    }
}

pub trait Reflectable {
    fn reflector<'a>(&'a self) -> &'a Reflector;
    fn mut_reflector<'a>(&'a mut self) -> &'a mut Reflector;
}

pub fn reflect_dom_object<T: Reflectable>
        (obj:     Box<T>,
         window:  &JSRef<window::Window>,
         wrap_fn: extern "Rust" fn(*mut JSContext, &JSRef<window::Window>, Box<T>) -> JS<T>)
         -> Temporary<T> {
    Temporary::new(wrap_fn(window.deref().get_cx(), window, obj))
}

#[allow(raw_pointer_deriving)]
#[deriving(Eq)]
pub struct Reflector {
    object: *mut JSObject,
}

impl Reflector {
    #[inline]
    pub fn get_jsobject(&self) -> *mut JSObject {
        self.object
    }

    pub fn set_jsobject(&mut self, object: *mut JSObject) {
        assert!(self.object.is_null());
        assert!(object.is_not_null());
        self.object = object;
    }

    /// Return a pointer to the memory location at which the JS reflector object is stored.
    /// Used by Temporary values to root the reflector, as required by the JSAPI rooting
    /// APIs.
    pub fn rootable<'a>(&'a mut self) -> &'a mut *mut JSObject {
        &mut self.object
    }

    pub fn new() -> Reflector {
        Reflector {
            object: ptr::mut_null(),
        }
    }
}

pub fn GetPropertyOnPrototype(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId, found: *mut bool,
                              vp: Option<JSMutableHandleValue>) -> bool {
    unsafe {
      //let proto = GetObjectProto(proxy);
      let mut proto = ptr::mut_null();
      let proto = MutableHandle {
          unnamed_field1: &mut proto,
      };
      if JS_GetPrototype(cx, proxy, proto) == 0 {
          return false;
      }
      if (*proto.unnamed_field1).is_null() {
          *found = false;
          return true;
      }
      let mut hasProp = 0;
      if JS_HasPropertyById(cx, proto.immut(), id, &mut hasProp) == 0 {
          return false;
      }
      *found = hasProp != 0;
      match vp {
          None | Some(_) if hasProp == 0 => true,
          Some(vp) => JS_ForwardGetPropertyTo(cx, proto.immut(), id, proxy, vp) != 0,
      }
  }
}

pub fn GetArrayIndexFromId(_cx: *mut JSContext, id: JSHandleId) -> Option<u32> {
    unsafe {
        if RUST_JSID_IS_INT(*id) != 0 {
            return Some(RUST_JSID_TO_INT(*id) as u32);
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

/*fn InternJSString(cx: *mut JSContext, chars: *libc::c_char) -> Option<JSHandleId> {
    unsafe {
        let s = JS_InternString(cx, chars);
        if s.is_not_null() {
            Some(RUST_INTERNED_STRING_TO_JSID(cx, s))
        } else {
            None
        }
    }
}*/

pub fn InitIds(_cx: *mut JSContext, _specs: &[JSPropertySpec], _ids: &mut [jsid]) -> bool {
    /*for (i, spec) in specs.iter().enumerate() {
        if spec.name.is_null() == true {
            return true;
        }
        match InternJSString(cx, spec.name) {
            Some(id) => ids[i] = id,
            None => {
                return false;
            }
        }
    }*/
    true
}

pub fn FindEnumStringIndex(cx: *mut JSContext,
                           v: JSVal,
                           values: &[&'static str]) -> Result<Option<uint>, ()> {
    unsafe {
        let v = Handle {
            unnamed_field1: &v,
        };
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
                value[j] as u16 == *chars.offset(j as int)
            })
        }))
    }
}

pub fn get_dictionary_property(cx: *mut JSContext,
                               object: JSHandleObject,
                               property: &str) -> Result<Option<JSVal>, ()> {
    use std::c_str::CString;
    fn has_property(cx: *mut JSContext, object: JSHandleObject, property: &CString,
                    found: &mut JSBool) -> bool {
        unsafe {
            property.with_ref(|s| {
                JS_HasProperty(cx, object, s, found) != 0
            })
        }
    }
    fn get_property(cx: *mut JSContext, object: JSHandleObject, property: &CString,
                    value: JSMutableHandleValue) -> bool {
        unsafe {
            property.with_ref(|s| {
                JS_GetProperty(cx, object, s, value) != 0
            })
        }
    }

    let property = property.to_c_str();
    if unsafe { (*object.unnamed_field1).is_null() } {
        return Ok(None);
    }

    let mut found: JSBool = 0;
    if !has_property(cx, object, &property, &mut found) {
        return Err(());
    }

    if found == 0 {
        return Ok(None);
    }

    let mut value = NullValue();
    let value = MutableHandle {
        unnamed_field1: &mut value,
    };
    if !get_property(cx, object, &property, value) {
        return Err(());
    }

    unsafe {
        Ok(Some(*value.unnamed_field1))
    }
}

pub fn HasPropertyOnPrototype(cx: *mut JSContext, proxy: JSHandleObject, id: JSHandleId) -> bool {
    //  MOZ_ASSERT(js::IsProxy(proxy) && js::GetProxyHandler(proxy) == handler);
    let mut found = false;
    return !GetPropertyOnPrototype(cx, proxy, id, &mut found, None) || found;
}

pub fn IsConvertibleToCallbackInterface(cx: *mut JSContext, obj: *mut JSObject) -> bool {
    unsafe {
        let obj = Handle {
            unnamed_field1: &obj,
        };
        JS_ObjectIsDate(cx, obj) == 0 && JS_ObjectIsRegExp(cx, obj) == 0
    }
}

pub fn CreateDOMGlobal(cx: *mut JSContext, class: *JSClass) -> *mut JSObject {
    unsafe {
        let obj = NewGlobalObject(cx, class, ptr::mut_null(), 1 /*DontFireOnNewGlobalHook*/);
        if obj.is_null() {
            return ptr::mut_null();
        }
        with_compartment(cx, obj, || {
            let globhandle = Handle {
                unnamed_field1: &obj,
            };
            JS_InitStandardClasses(cx, globhandle);
        });
        initialize_global(obj);
        obj
    }
}

pub extern fn wrap_for_same_compartment(cx: *mut JSContext, obj: *mut JSObject) -> *mut JSObject {
    unsafe {
        JS_ObjectToOuterObject(cx, obj)
    }
}

pub extern fn pre_wrap(cx: *mut JSContext, _scope: *mut JSObject,
                       obj: *mut JSObject, _flags: c_uint) -> *mut JSObject {
    unsafe {
        JS_ObjectToOuterObject(cx, obj)
    }
}

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
        win.deref().browser_context.get_ref().window_proxy()
    }
}

/// Returns the global object of the realm that the given JS object was created in.
pub fn global_object_for_js_object(obj: *mut JSObject) -> JS<window::Window> {
    let obj = Handle {
        unnamed_field1: &obj
    };
    unsafe {
        let global = GetGlobalForObjectCrossCompartment(*obj);
        let clasp = JS_GetClass(global);
        assert!(((*clasp).flags & (JSCLASS_IS_DOMJSCLASS | JSCLASS_IS_GLOBAL)) != 0);
        FromJSValConvertible::from_jsval(ptr::mut_null(), ObjectOrNullValue(global), ())
            .ok().expect("found DOM global that doesn't unwrap to Window")
    }
}

fn cx_for_dom_reflector(obj: *mut JSObject) -> *mut JSContext {
    let win = global_object_for_js_object(obj).root();
    let js_info = win.deref().page().js_info();
    match *js_info {
        Some(ref info) => info.js_context.deref().deref().ptr,
        None => fail!("no JS context for DOM global")
    }
}

pub fn cx_for_dom_object<T: Reflectable>(obj: &T) -> *mut JSContext {
    cx_for_dom_reflector(obj.reflector().get_jsobject())
}

/// Check if an element name is valid. See http://www.w3.org/TR/xml/#NT-Name
/// for details.
#[deriving(Eq)]
pub enum XMLName {
    QName,
    Name,
    InvalidXMLName
}

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
