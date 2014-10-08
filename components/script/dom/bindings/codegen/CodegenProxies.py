# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from CodegenGeneric import (
    Argument,
    CGAbstractExternMethod,
    CGAbstractMethod,
    CGGeneric,
    CGIfWrapper,
    CGIndenter,
    CGThing,
    stripTrailingWhitespace,
    toStringBool,
)

from CodegenRust import (
    CGCallGenerator,
    CGPerSignatureCall,
    DOMClass,
    FakeArgument,
    getJSToNativeConversionTemplate,
    instantiateJSToNativeConversionTemplate,
    MakeNativeName,
    wrapForType,
)

from WebIDL import (
    BuiltinTypes,
    IDLBuiltinType,
)

class CGDOMJSProxyHandlerDOMClass(CGThing):
    def __init__(self, descriptor):
        CGThing.__init__(self)
        self.descriptor = descriptor

    def define(self):
        return """
static Class: DOMClass = """ + DOMClass(self.descriptor) + """;

"""


class CGProxySpecialOperation(CGPerSignatureCall):
    """
    Base class for classes for calling an indexed or named special operation
    (don't use this directly, use the derived classes below).
    """
    def __init__(self, descriptor, operation):
        nativeName = MakeNativeName(operation)
        operation = descriptor.operations[operation]
        assert len(operation.signatures()) == 1
        signature = operation.signatures()[0]

        (returnType, arguments) = signature

        # We pass len(arguments) as the final argument so that the
        # CGPerSignatureCall won't do any argument conversion of its own.
        CGPerSignatureCall.__init__(self, returnType, "", arguments, nativeName,
                                    False, descriptor, operation,
                                    len(arguments))

        if operation.isSetter() or operation.isCreator():
            # arguments[0] is the index or name of the item that we're setting.
            argument = arguments[1]
            template, _, declType, needsRooting = getJSToNativeConversionTemplate(
                argument.type, descriptor, treatNullAs=argument.treatNullAs)
            templateValues = {
                "val": "(*desc).value",
            }
            self.cgRoot.prepend(instantiateJSToNativeConversionTemplate(
                  template, templateValues, declType, argument.identifier.name,
                  needsRooting))
        elif operation.isGetter():
            self.cgRoot.prepend(CGGeneric("let mut found = false;"))

    def getArguments(self):
        def process(arg):
            argVal = arg.identifier.name
            if arg.type.isGeckoInterface() and not arg.type.unroll().inner.isCallback():
                argVal += ".root_ref()"
            return argVal
        args = [(a, process(a)) for a in self.arguments]
        if self.idlNode.isGetter():
            args.append((FakeArgument(BuiltinTypes[IDLBuiltinType.Types.boolean],
                                      self.idlNode),
                         "&mut found"))
        return args

    def wrap_return_value(self):
        if not self.idlNode.isGetter() or self.templateValues is None:
            return ""

        wrap = CGGeneric(wrapForType(**self.templateValues))
        wrap = CGIfWrapper(wrap, "found")
        return "\n" + wrap.define()

class CGProxyIndexedGetter(CGProxySpecialOperation):
    """
    Class to generate a call to an indexed getter. If templateValues is not None
    the returned value will be wrapped with wrapForType using templateValues.
    """
    def __init__(self, descriptor, templateValues=None):
        self.templateValues = templateValues
        CGProxySpecialOperation.__init__(self, descriptor, 'IndexedGetter')

class CGProxyIndexedSetter(CGProxySpecialOperation):
    """
    Class to generate a call to an indexed setter.
    """
    def __init__(self, descriptor):
        CGProxySpecialOperation.__init__(self, descriptor, 'IndexedSetter')

class CGProxyNamedGetter(CGProxySpecialOperation):
    """
    Class to generate a call to an named getter. If templateValues is not None
    the returned value will be wrapped with wrapForType using templateValues.
    """
    def __init__(self, descriptor, templateValues=None):
        self.templateValues = templateValues
        CGProxySpecialOperation.__init__(self, descriptor, 'NamedGetter')

class CGProxyNamedSetter(CGProxySpecialOperation):
    """
    Class to generate a call to a named setter.
    """
    def __init__(self, descriptor):
        CGProxySpecialOperation.__init__(self, descriptor, 'NamedSetter')

class CGProxyUnwrap(CGAbstractMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSObject', 'obj')]
        CGAbstractMethod.__init__(self, descriptor, "UnwrapProxy", '*const ' + descriptor.concreteType, args, alwaysInline=True)

    def definition_body(self):
        return CGGeneric("""/*if (xpc::WrapperFactory::IsXrayWrapper(obj)) {
  obj = js::UnwrapObject(obj);
}*/
//MOZ_ASSERT(IsProxy(obj));
let box_ = GetProxyPrivate(obj).to_private() as *const %s;
return box_;""" % self.descriptor.concreteType)

class CGDOMJSProxyHandler_getOwnPropertyDescriptor(CGAbstractExternMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSContext', 'cx'), Argument('*mut JSObject', 'proxy'),
                Argument('jsid', 'id'), Argument('bool', 'set'),
                Argument('*mut JSPropertyDescriptor', 'desc')]
        CGAbstractExternMethod.__init__(self, descriptor, "getOwnPropertyDescriptor",
                                        "bool", args)
        self.descriptor = descriptor
    def getBody(self):
        indexedGetter = self.descriptor.operations['IndexedGetter']
        indexedSetter = self.descriptor.operations['IndexedSetter']

        setOrIndexedGet = ""
        if indexedGetter or indexedSetter:
            setOrIndexedGet += "let index = GetArrayIndexFromId(cx, id);\n"

        if indexedGetter:
            readonly = toStringBool(self.descriptor.operations['IndexedSetter'] is None)
            fillDescriptor = "FillPropertyDescriptor(&mut *desc, proxy, %s);\nreturn true;" % readonly
            templateValues = {'jsvalRef': '(*desc).value', 'successCode': fillDescriptor}
            get = ("if index.is_some() {\n" +
                   "  let index = index.unwrap();\n" +
                   "  let this = UnwrapProxy(proxy);\n" +
                   "  let this = JS::from_raw(this);\n" +
                   "  let this = this.root();\n" +
                   CGIndenter(CGProxyIndexedGetter(self.descriptor, templateValues)).define() + "\n" +
                   "}\n")

        if indexedSetter or self.descriptor.operations['NamedSetter']:
            setOrIndexedGet += "if set != 0 {\n"
            if indexedSetter:
                setOrIndexedGet += ("  if index.is_some() {\n" +
                                    "    let index = index.unwrap();\n")
                if not 'IndexedCreator' in self.descriptor.operations:
                    # FIXME need to check that this is a 'supported property index'
                    assert False
                setOrIndexedGet += ("    FillPropertyDescriptor(&mut *desc, proxy, false);\n" +
                                    "    return true;\n" +
                                    "  }\n")
            if self.descriptor.operations['NamedSetter']:
                setOrIndexedGet += "  if RUST_JSID_IS_STRING(id) {\n"
                if not 'NamedCreator' in self.descriptor.operations:
                    # FIXME need to check that this is a 'supported property name'
                    assert False
                setOrIndexedGet += ("    FillPropertyDescriptor(&mut *desc, proxy, false);\n" +
                                    "    return true;\n" +
                                    "  }\n")
            setOrIndexedGet += "}"
            if indexedGetter:
                setOrIndexedGet += (" else {\n" +
                                    CGIndenter(CGGeneric(get)).define() +
                                    "}")
            setOrIndexedGet += "\n\n"
        elif indexedGetter:
            setOrIndexedGet += ("if !set {\n" +
                                CGIndenter(CGGeneric(get)).define() +
                                "}\n\n")

        namedGetter = self.descriptor.operations['NamedGetter']
        if namedGetter:
            readonly = toStringBool(self.descriptor.operations['NamedSetter'] is None)
            fillDescriptor = "FillPropertyDescriptor(&mut *desc, proxy, %s);\nreturn true;" % readonly
            templateValues = {'jsvalRef': '(*desc).value', 'successCode': fillDescriptor}
            # Once we start supporting OverrideBuiltins we need to make
            # ResolveOwnProperty or EnumerateOwnProperties filter out named
            # properties that shadow prototype properties.
            namedGet = ("\n" +
                        "if !set && RUST_JSID_IS_STRING(id) != 0 && !HasPropertyOnPrototype(cx, proxy, id) {\n" +
                        "  let name = jsid_to_str(cx, id);\n" +
                        "  let this = UnwrapProxy(proxy);\n" +
                        "  let this = JS::from_raw(this);\n" +
                        "  let this = this.root();\n" +
                        CGIndenter(CGProxyNamedGetter(self.descriptor, templateValues)).define() + "\n" +
                        "}\n")
        else:
            namedGet = ""

        return setOrIndexedGet + """let expando: *mut JSObject = GetExpandoObject(proxy);
//if (!xpc::WrapperFactory::IsXrayWrapper(proxy) && (expando = GetExpandoObject(proxy))) {
if expando.is_not_null() {
  let flags = if set { JSRESOLVE_ASSIGNING } else { 0 } | JSRESOLVE_QUALIFIED;
  if JS_GetPropertyDescriptorById(cx, expando, id, flags, desc) == 0 {
    return false;
  }
  if (*desc).obj.is_not_null() {
    // Pretend the property lives on the wrapper.
    (*desc).obj = proxy;
    return true;
  }
}
""" + namedGet + """
(*desc).obj = ptr::null_mut();
return true;"""

    def definition_body(self):
        return CGGeneric(self.getBody())

class CGDOMJSProxyHandler_defineProperty(CGAbstractExternMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSContext', 'cx'), Argument('*mut JSObject', 'proxy'),
                Argument('jsid', 'id'),
                Argument('*const JSPropertyDescriptor', 'desc')]
        CGAbstractExternMethod.__init__(self, descriptor, "defineProperty", "bool", args)
        self.descriptor = descriptor
    def getBody(self):
        set = ""

        indexedSetter = self.descriptor.operations['IndexedSetter']
        if indexedSetter:
            if not (self.descriptor.operations['IndexedCreator'] is indexedSetter):
                raise TypeError("Can't handle creator that's different from the setter")
            set += ("let index = GetArrayIndexFromId(cx, id);\n" +
                    "if index.is_some() {\n" +
                    "  let index = index.unwrap();\n" +
                    "  let this = UnwrapProxy(proxy);\n" +
                    "  let this = JS::from_raw(this);\n" +
                    "  let this = this.root();\n" +
                    CGIndenter(CGProxyIndexedSetter(self.descriptor)).define() +
                    "  return true;\n" +
                    "}\n")
        elif self.descriptor.operations['IndexedGetter']:
            set += ("if GetArrayIndexFromId(cx, id).is_some() {\n" +
                    "  return false;\n" +
                    "  //return ThrowErrorMessage(cx, MSG_NO_PROPERTY_SETTER, \"%s\");\n" +
                    "}\n") % self.descriptor.name

        namedSetter = self.descriptor.operations['NamedSetter']
        if namedSetter:
            if not self.descriptor.operations['NamedCreator'] is namedSetter:
                raise TypeError("Can't handle creator that's different from the setter")
            set += ("if RUST_JSID_IS_STRING(id) != 0 {\n" +
                    "  let name = jsid_to_str(cx, id);\n" +
                    "  let this = UnwrapProxy(proxy);\n" +
                    "  let this = JS::from_raw(this);\n" +
                    "  let this = this.root();\n" +
                    CGIndenter(CGProxyNamedSetter(self.descriptor)).define() + "\n" +
                    "}\n")
        elif self.descriptor.operations['NamedGetter']:
            set += ("if RUST_JSID_IS_STRING(id) {\n" +
                    "  let name = jsid_to_str(cx, id);\n" +
                    "  let this = UnwrapProxy(proxy);\n" +
                    "  let this = JS::from_raw(this);\n" +
                    "  let this = this.root();\n" +
                    CGIndenter(CGProxyNamedGetter(self.descriptor)).define() +
                    "  if (found) {\n"
                    "    return false;\n" +
                    "    //return ThrowErrorMessage(cx, MSG_NO_PROPERTY_SETTER, \"%s\");\n" +
                    "  }\n" +
                    "  return true;\n"
                    "}\n") % (self.descriptor.name)
        return set + """return proxyhandler::defineProperty_(%s);""" % ", ".join(a.name for a in self.args)

    def definition_body(self):
        return CGGeneric(self.getBody())

class CGDOMJSProxyHandler_hasOwn(CGAbstractExternMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSContext', 'cx'), Argument('*mut JSObject', 'proxy'),
                Argument('jsid', 'id'), Argument('*mut bool', 'bp')]
        CGAbstractExternMethod.__init__(self, descriptor, "hasOwn", "bool", args)
        self.descriptor = descriptor
    def getBody(self):
        indexedGetter = self.descriptor.operations['IndexedGetter']
        if indexedGetter:
            indexed = ("let index = GetArrayIndexFromId(cx, id);\n" +
                       "if index.is_some() {\n" +
                       "  let index = index.unwrap();\n" +
                       "  let this = UnwrapProxy(proxy);\n" +
                       "  let this = JS::from_raw(this);\n" +
                       "  let this = this.root();\n" +
                       CGIndenter(CGProxyIndexedGetter(self.descriptor)).define() + "\n" +
                       "  *bp = found;\n" +
                       "  return true;\n" +
                       "}\n\n")
        else:
            indexed = ""

        namedGetter = self.descriptor.operations['NamedGetter']
        if namedGetter:
            named = ("if RUST_JSID_IS_STRING(id) != 0 && !HasPropertyOnPrototype(cx, proxy, id) {\n" +
                     "  let name = jsid_to_str(cx, id);\n" +
                     "  let this = UnwrapProxy(proxy);\n" +
                     "  let this = JS::from_raw(this);\n" +
                     "  let this = this.root();\n" +
                     CGIndenter(CGProxyNamedGetter(self.descriptor)).define() + "\n" +
                     "  *bp = found;\n"
                     "  return true;\n"
                     "}\n" +
                     "\n")
        else:
            named = ""

        return indexed + """let expando: *mut JSObject = GetExpandoObject(proxy);
if expando.is_not_null() {
  let mut b: JSBool = 1;
  let ok = JS_HasPropertyById(cx, expando, id, &mut b) != 0;
  *bp = b != 0;
  if !ok || *bp {
    return ok;
  }
}

""" + named + """*bp = false;
return true;"""

    def definition_body(self):
        return CGGeneric(self.getBody())

class CGDOMJSProxyHandler_get(CGAbstractExternMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSContext', 'cx'), Argument('*mut JSObject', 'proxy'),
                Argument('*mut JSObject', 'receiver'), Argument('jsid', 'id'),
                Argument('*mut JSVal', 'vp')]
        CGAbstractExternMethod.__init__(self, descriptor, "get", "bool", args)
        self.descriptor = descriptor
    def getBody(self):
        getFromExpando = """let expando = GetExpandoObject(proxy);
if expando.is_not_null() {
  let mut hasProp = 0;
  if JS_HasPropertyById(cx, expando, id, &mut hasProp) == 0 {
    return false;
  }

  if hasProp != 0 {
    return JS_GetPropertyById(cx, expando, id, vp) != 0;
  }
}"""

        templateValues = {
            'jsvalRef': '*vp',
            'successCode': 'return true;',
        }

        indexedGetter = self.descriptor.operations['IndexedGetter']
        if indexedGetter:
            getIndexedOrExpando = ("let index = GetArrayIndexFromId(cx, id);\n" +
                                   "if index.is_some() {\n" +
                                   "  let index = index.unwrap();\n" +
                                   "  let this = UnwrapProxy(proxy);\n" +
                                   "  let this = JS::from_raw(this);\n" +
                                   "  let this = this.root();\n" +
                                   CGIndenter(CGProxyIndexedGetter(self.descriptor, templateValues)).define())
            getIndexedOrExpando += """
  // Even if we don't have this index, we don't forward the
  // get on to our expando object.
} else {
  %s
}
""" % (stripTrailingWhitespace(getFromExpando.replace('\n', '\n  ')))
        else:
            getIndexedOrExpando = getFromExpando + "\n"

        namedGetter = self.descriptor.operations['NamedGetter']
        if namedGetter and False: #XXXjdm unfinished
            getNamed = ("if (JSID_IS_STRING(id)) {\n" +
                        "  let name = jsid_to_str(cx, id);\n" +
                        "  let this = UnwrapProxy(proxy);\n" +
                        "  let this = JS::from_raw(this);\n" +
                        "  let this = this.root();\n" +
                        CGIndenter(CGProxyNamedGetter(self.descriptor, templateValues)).define() +
                        "}\n") % (self.descriptor.concreteType)
        else:
            getNamed = ""

        return """//MOZ_ASSERT(!xpc::WrapperFactory::IsXrayWrapper(proxy),
            //"Should not have a XrayWrapper here");

%s
let mut found = false;
if !GetPropertyOnPrototype(cx, proxy, id, &mut found, vp) {
  return false;
}

if found {
  return true;
}
%s
*vp = UndefinedValue();
return true;""" % (getIndexedOrExpando, getNamed)

    def definition_body(self):
        return CGGeneric(self.getBody())

class CGDOMJSProxyHandler_obj_toString(CGAbstractExternMethod):
    def __init__(self, descriptor):
        args = [Argument('*mut JSContext', 'cx'), Argument('*mut JSObject', 'proxy')]
        CGAbstractExternMethod.__init__(self, descriptor, "obj_toString", "*mut JSString", args)
        self.descriptor = descriptor
    def getBody(self):
        stringifier = self.descriptor.operations['Stringifier']
        if stringifier:
            nativeName = MakeNativeName(stringifier.identifier.name)
            signature = stringifier.signatures()[0]
            returnType = signature[0]
            extendedAttributes = self.descriptor.getExtendedAttributes(stringifier)
            infallible = 'infallible' in extendedAttributes
            if not infallible:
                error = CGGeneric(
                    ('ThrowMethodFailedWithDetails(cx, rv, "%s", "toString");\n' +
                     "return NULL;") % self.descriptor.interface.identifier.name)
            else:
                error = None
            call = CGCallGenerator(error, [], "", returnType, extendedAttributes, self.descriptor, nativeName, False, object="UnwrapProxy(proxy)")
            return call.define() + """

JSString* jsresult;
return xpc_qsStringToJsstring(cx, result, &jsresult) ? jsresult : NULL;""" 

        return """let s = "%s".to_c_str();
  _obj_toString(cx, s.as_ptr())""" % self.descriptor.name

    def definition_body(self):
        return CGGeneric(self.getBody())
