# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import re
import string

from CodegenGeneric import (
    Argument,
    CGClass,
    CGGeneric,
    CGIfElseWrapper,
    CGIfWrapper,
    CGIndenter,
    CGList,
    CGWrapper,
    ClassBase,
    ClassConstructor,
    ClassMethod,
)

from CodegenRust import (
    CGNativeMember,
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


class CGCallback(CGClass):
    def __init__(self, idlObject, descriptorProvider, baseName, methods,
                 getters=[], setters=[]):
        self.baseName = baseName
        self._deps = idlObject.getDeps()
        name = idlObject.identifier.name
        # For our public methods that needThisHandling we want most of the
        # same args and the same return type as what CallbackMember
        # generates.  So we want to take advantage of all its
        # CGNativeMember infrastructure, but that infrastructure can't deal
        # with templates and most especially template arguments.  So just
        # cheat and have CallbackMember compute all those things for us.
        realMethods = []
        for method in methods:
            if not method.needThisHandling:
                realMethods.append(method)
            else:
                realMethods.extend(self.getMethodImpls(method))
        CGClass.__init__(self, name,
                         bases=[ClassBase(baseName)],
                         constructors=self.getConstructors(),
                         methods=realMethods+getters+setters,
                         decorators="#[deriving(PartialEq,Clone)]#[jstraceable]")

    def getConstructors(self):
        return [ClassConstructor(
            [Argument("*mut JSObject", "aCallback")],
            bodyInHeader=True,
            visibility="pub",
            explicit=False,
            baseConstructors=[
                "%s::new(aCallback)" % self.baseName
                ])]

    def getMethodImpls(self, method):
        assert method.needThisHandling
        args = list(method.args)
        # Strip out the JSContext*/JSObject* args
        # that got added.
        assert args[0].name == "cx" and args[0].argType == "*mut JSContext"
        assert args[1].name == "aThisObj" and args[1].argType == "*mut JSObject"
        args = args[2:]
        # Record the names of all the arguments, so we can use them when we call
        # the private method.
        argnames = [arg.name for arg in args]
        argnamesWithThis = ["s.GetContext()", "thisObjJS"] + argnames
        argnamesWithoutThis = ["s.GetContext()", "ptr::null_mut()"] + argnames
        # Now that we've recorded the argnames for our call to our private
        # method, insert our optional argument for deciding whether the
        # CallSetup should re-throw exceptions on aRv.
        args.append(Argument("ExceptionHandling", "aExceptionHandling",
                             "ReportExceptions"))

        args[0] = Argument(args[0].argType, args[0].name, args[0].default)
        method.args[2] = args[0]

        # And now insert our template argument.
        argsWithoutThis = list(args)
        args.insert(0, Argument("JSRef<T>",  "thisObj"))

        # And the self argument
        method.args.insert(0, Argument(None, "self"))
        args.insert(0, Argument(None, "self"))
        argsWithoutThis.insert(0, Argument(None, "self"))

        setupCall = ("let s = CallSetup::new(self, aExceptionHandling);\n"
                     "if s.GetContext().is_null() {\n"
                     "  return Err(FailureUnknown);\n"
                     "}\n")

        bodyWithThis = string.Template(
            setupCall+
            "let thisObjJS = WrapCallThisObject(s.GetContext(), thisObj);\n"
            "if thisObjJS.is_null() {\n"
            "  return Err(FailureUnknown);\n"
            "}\n"
            "return ${methodName}(${callArgs});").substitute({
                "callArgs" : ", ".join(argnamesWithThis),
                "methodName": 'self.' + method.name,
                })
        bodyWithoutThis = string.Template(
            setupCall +
            "return ${methodName}(${callArgs});").substitute({
                "callArgs" : ", ".join(argnamesWithoutThis),
                "methodName": 'self.' + method.name,
                })
        return [ClassMethod(method.name+'_', method.returnType, args,
                            bodyInHeader=True,
                            templateArgs=["T: Reflectable"],
                            body=bodyWithThis,
                            visibility='pub'),
                ClassMethod(method.name+'__', method.returnType, argsWithoutThis,
                            bodyInHeader=True,
                            body=bodyWithoutThis,
                            visibility='pub'),
                method]

    def deps(self):
        return self._deps

# We're always fallible
def callbackGetterName(attr):
    return "Get" + MakeNativeName(attr.identifier.name)

def callbackSetterName(attr):
    return "Set" + MakeNativeName(attr.identifier.name)

class CGCallbackFunction(CGCallback):
    def __init__(self, callback, descriptorProvider):
        CGCallback.__init__(self, callback, descriptorProvider,
                            "CallbackFunction",
                            methods=[CallCallback(callback, descriptorProvider)])

    def getConstructors(self):
        return CGCallback.getConstructors(self)

class CGCallbackFunctionImpl(CGGeneric):
    def __init__(self, callback):
        impl = string.Template("""impl CallbackContainer for ${type} {
    fn new(callback: *mut JSObject) -> ${type} {
        ${type}::new(callback)
    }

    fn callback(&self) -> *mut JSObject {
        self.parent.callback()
    }
}

impl ToJSValConvertible for ${type} {
    fn to_jsval(&self, cx: *mut JSContext) -> JSVal {
        self.callback().to_jsval(cx)
    }
}
""").substitute({"type": callback.name})
        CGGeneric.__init__(self, impl)

class CGCallbackInterface(CGCallback):
    def __init__(self, descriptor):
        iface = descriptor.interface
        attrs = [m for m in iface.members if m.isAttr() and not m.isStatic()]
        getters = [CallbackGetter(a, descriptor) for a in attrs]
        setters = [CallbackSetter(a, descriptor) for a in attrs
                   if not a.readonly]
        methods = [m for m in iface.members
                   if m.isMethod() and not m.isStatic() and not m.isIdentifierLess()]
        methods = [CallbackOperation(m, sig, descriptor) for m in methods
                   for sig in m.signatures()]
        assert not iface.isJSImplemented() or not iface.ctor()
        CGCallback.__init__(self, iface, descriptor, "CallbackInterface",
                            methods, getters=getters, setters=setters)

class FakeMember():
    def __init__(self):
        self.treatNullAs = "Default"
    def isStatic(self):
        return False
    def isAttr(self):
        return False
    def isMethod(self):
        return False
    def getExtendedAttribute(self, name):
        return None

class CallbackMember(CGNativeMember):
    def __init__(self, sig, name, descriptorProvider, needThisHandling, rethrowContentException=False):
        """
        needThisHandling is True if we need to be able to accept a specified
        thisObj, False otherwise.
        """
        assert not rethrowContentException or not needThisHandling

        self.retvalType = sig[0]
        self.originalSig = sig
        args = sig[1]
        self.argCount = len(args)
        if self.argCount > 0:
            # Check for variadic arguments
            lastArg = args[self.argCount-1]
            if lastArg.variadic:
                self.argCountStr = (
                    "(%d - 1) + %s.Length()" % (self.argCount,
                                                lastArg.identifier.name))
            else:
                self.argCountStr = "%d" % self.argCount
        self.needThisHandling = needThisHandling
        # If needThisHandling, we generate ourselves as private and the caller
        # will handle generating public versions that handle the "this" stuff.
        visibility = "priv" if needThisHandling else "pub"
        self.rethrowContentException = rethrowContentException
        # We don't care, for callback codegen, whether our original member was
        # a method or attribute or whatnot.  Just always pass FakeMember()
        # here.
        CGNativeMember.__init__(self, descriptorProvider, FakeMember(),
                                name, (self.retvalType, args),
                                extendedAttrs={},
                                passJSBitsAsNeeded=False,
                                visibility=visibility,
                                jsObjectsArePtr=True)
        # We have to do all the generation of our body now, because
        # the caller relies on us throwing if we can't manage it.
        self.exceptionCode= "return Err(FailureUnknown);\n"
        self.body = self.getImpl()

    def getImpl(self):
        replacements = {
            "declRval": self.getRvalDecl(),
            "returnResult": self.getResultConversion(),
            "convertArgs": self.getArgConversions(),
            "doCall": self.getCall(),
            "setupCall": self.getCallSetup(),
            }
        if self.argCount > 0:
            replacements["argCount"] = self.argCountStr
            replacements["argvDecl"] = string.Template(
                "let mut argv = Vec::from_elem(${argCount}, UndefinedValue());\n"
                ).substitute(replacements)
        else:
            # Avoid weird 0-sized arrays
            replacements["argvDecl"] = ""

        # Newlines and semicolons are in the values
        pre = string.Template(
            "${setupCall}"
            "${declRval}"
            "${argvDecl}").substitute(replacements)
        body = string.Template(
            "${convertArgs}"
            "${doCall}"
            "${returnResult}").substitute(replacements)
        return CGList([
            CGGeneric(pre),
            CGWrapper(CGIndenter(CGGeneric(body)),
                      pre="with_compartment(cx, self.parent.callback(), || {\n",
                      post="})")
        ], "\n").define()

    def getResultConversion(self):
        replacements = {
            "val": "rval",
            "declName": "rvalDecl",
        }

        template, _, declType, needsRooting = getJSToNativeConversionTemplate(
            self.retvalType,
            self.descriptorProvider,
            exceptionCode=self.exceptionCode,
            isCallbackReturnValue="Callback",
            # XXXbz we should try to do better here
            sourceDescription="return value")

        convertType = instantiateJSToNativeConversionTemplate(
            template, replacements, declType, "rvalDecl", needsRooting)

        assignRetval = string.Template(
            self.getRetvalInfo(self.retvalType,
                               False)[1]).substitute(replacements)
        return convertType.define() + "\n" + assignRetval + "\n"

    def getArgConversions(self):
        # Just reget the arglist from self.originalSig, because our superclasses
        # just have way to many members they like to clobber, so I can't find a
        # safe member name to store it in.
        argConversions = [self.getArgConversion(i, arg) for (i, arg)
                          in enumerate(self.originalSig[1])]
        # Do them back to front, so our argc modifications will work
        # correctly, because we examine trailing arguments first.
        argConversions.reverse();
        # Wrap each one in a scope so that any locals it has don't leak out, and
        # also so that we can just "break;" for our successCode.
        argConversions = [CGWrapper(CGIndenter(CGGeneric(c)),
                                    pre="loop {\n",
                                    post="\nbreak;}\n")
                          for c in argConversions]
        if self.argCount > 0:
            argConversions.insert(0, self.getArgcDecl())
        # And slap them together.
        return CGList(argConversions, "\n\n").define() + "\n\n"

    def getArgConversion(self, i, arg):
        argval = arg.identifier.name

        if arg.variadic:
            argval = argval + "[idx]"
            jsvalIndex = "%d + idx" % i
        else:
            jsvalIndex = "%d" % i
            if arg.optional and not arg.defaultValue:
                argval += ".clone().unwrap()"

        conversion = wrapForType("*argv.get_mut(%s)" % jsvalIndex,
                result=argval,
                successCode="continue;" if arg.variadic else "break;")
        if arg.variadic:
            conversion = string.Template(
                "for (uint32_t idx = 0; idx < ${arg}.Length(); ++idx) {\n" +
                CGIndenter(CGGeneric(conversion)).define() + "\n"
                "}\n"
                "break;").substitute({ "arg": arg.identifier.name })
        elif arg.optional and not arg.defaultValue:
            conversion = (
                CGIfWrapper(CGGeneric(conversion),
                            "%s.is_some()" % arg.identifier.name).define() +
                " else if (argc == %d) {\n"
                "  // This is our current trailing argument; reduce argc\n"
                "  argc -= 1;\n"
                "} else {\n"
                "  *argv.get_mut(%d) = UndefinedValue();\n"
                "}" % (i+1, i))
        return conversion

    def getArgs(self, returnType, argList):
        args = CGNativeMember.getArgs(self, returnType, argList)
        if not self.needThisHandling:
            # Since we don't need this handling, we're the actual method that
            # will be called, so we need an aRethrowExceptions argument.
            if self.rethrowContentException:
                args.append(Argument("JSCompartment*", "aCompartment", "nullptr"))
            else:
                args.append(Argument("ExceptionHandling", "aExceptionHandling",
                                     "ReportExceptions"))
            return args
        # We want to allow the caller to pass in a "this" object, as
        # well as a JSContext.
        return [Argument("*mut JSContext", "cx"),
                Argument("*mut JSObject", "aThisObj")] + args

    def getCallSetup(self):
        if self.needThisHandling:
            # It's been done for us already
            return ""
        callSetup = "CallSetup s(CallbackPreserveColor(), aRv"
        if self.rethrowContentException:
            # getArgs doesn't add the aExceptionHandling argument but does add
            # aCompartment for us.
            callSetup += ", RethrowContentExceptions, aCompartment"
        else:
            callSetup += ", aExceptionHandling"
        callSetup += ");"
        return string.Template(
            "${callSetup}\n"
            "JSContext* cx = s.GetContext();\n"
            "if (!cx) {\n"
            "  return Err(FailureUnknown);\n"
            "}\n").substitute({
                "callSetup": callSetup,
            })

    def getArgcDecl(self):
        return CGGeneric("let mut argc = %su32;" % self.argCountStr);

    @staticmethod
    def ensureASCIIName(idlObject):
        type = "attribute" if idlObject.isAttr() else "operation"
        if re.match("[^\x20-\x7E]", idlObject.identifier.name):
            raise SyntaxError('Callback %s name "%s" contains non-ASCII '
                              "characters.  We can't handle that.  %s" %
                              (type, idlObject.identifier.name,
                               idlObject.location))
        if re.match('"', idlObject.identifier.name):
            raise SyntaxError("Callback %s name '%s' contains "
                              "double-quote character.  We can't handle "
                              "that.  %s" %
                              (type, idlObject.identifier.name,
                               idlObject.location))

class CallbackMethod(CallbackMember):
    def __init__(self, sig, name, descriptorProvider, needThisHandling, rethrowContentException=False):
        CallbackMember.__init__(self, sig, name, descriptorProvider,
                                needThisHandling, rethrowContentException)
    def getRvalDecl(self):
        return "let mut rval = UndefinedValue();\n"

    def getCall(self):
        replacements = {
            "thisObj": self.getThisObj(),
            "getCallable": self.getCallableDecl()
            }
        if self.argCount > 0:
            replacements["argv"] = "argv.as_mut_ptr()"
            replacements["argc"] = "argc"
        else:
            replacements["argv"] = "nullptr"
            replacements["argc"] = "0"
        return string.Template("${getCallable}"
                "let ok = unsafe {\n"
                "  JS_CallFunctionValue(cx, ${thisObj}, callable,\n"
                "                       ${argc}, ${argv}, &mut rval)\n"
                "};\n"
                "if ok == 0 {\n"
                "  return Err(FailureUnknown);\n"
                "}\n").substitute(replacements)

class CallCallback(CallbackMethod):
    def __init__(self, callback, descriptorProvider):
        CallbackMethod.__init__(self, callback.signatures()[0], "Call",
                                descriptorProvider, needThisHandling=True)

    def getThisObj(self):
        return "aThisObj"

    def getCallableDecl(self):
        return "let callable = ObjectValue(unsafe {&*self.parent.callback()});\n";

class CallbackOperationBase(CallbackMethod):
    """
    Common class for implementing various callback operations.
    """
    def __init__(self, signature, jsName, nativeName, descriptor, singleOperation, rethrowContentException=False):
        self.singleOperation = singleOperation
        self.methodName = jsName
        CallbackMethod.__init__(self, signature, nativeName, descriptor, singleOperation, rethrowContentException)

    def getThisObj(self):
        if not self.singleOperation:
            return "self.parent.callback()"
        # This relies on getCallableDecl declaring a boolean
        # isCallable in the case when we're a single-operation
        # interface.
        return "if isCallable { aThisObj } else { self.parent.callback() }"

    def getCallableDecl(self):
        replacements = {
            "methodName": self.methodName
        }
        getCallableFromProp = string.Template(
                'match self.parent.GetCallableProperty(cx, "${methodName}") {\n'
                '  Err(_) => return Err(FailureUnknown),\n'
                '  Ok(callable) => callable,\n'
                '}').substitute(replacements)
        if not self.singleOperation:
            return 'JS::Rooted<JS::Value> callable(cx);\n' + getCallableFromProp
        return (
            'let isCallable = unsafe { JS_ObjectIsCallable(cx, self.parent.callback()) != 0 };\n'
            'let callable =\n' +
            CGIndenter(
                CGIfElseWrapper('isCallable',
                                CGGeneric('unsafe { ObjectValue(&*self.parent.callback()) }'),
                                CGGeneric(getCallableFromProp))).define() + ';\n')

class CallbackOperation(CallbackOperationBase):
    """
    Codegen actual WebIDL operations on callback interfaces.
    """
    def __init__(self, method, signature, descriptor):
        self.ensureASCIIName(method)
        jsName = method.identifier.name
        CallbackOperationBase.__init__(self, signature,
                                       jsName, MakeNativeName(jsName),
                                       descriptor, descriptor.interface.isSingleOperationInterface(),
                                       rethrowContentException=descriptor.interface.isJSImplemented())

class CallbackGetter(CallbackMember):
    def __init__(self, attr, descriptor):
        self.ensureASCIIName(attr)
        self.attrName = attr.identifier.name
        CallbackMember.__init__(self,
                                (attr.type, []),
                                callbackGetterName(attr),
                                descriptor,
                                needThisHandling=False,
                                rethrowContentException=descriptor.interface.isJSImplemented())

    def getRvalDecl(self):
        return "JS::Rooted<JS::Value> rval(cx, JS::UndefinedValue());\n"

    def getCall(self):
        replacements = {
            "attrName": self.attrName
        }
        return string.Template(
            'if (!JS_GetProperty(cx, mCallback, "${attrName}", &rval)) {\n'
            '  return Err(FailureUnknown);\n'
            '}\n').substitute(replacements);

class CallbackSetter(CallbackMember):
    def __init__(self, attr, descriptor):
        self.ensureASCIIName(attr)
        self.attrName = attr.identifier.name
        CallbackMember.__init__(self,
                                (BuiltinTypes[IDLBuiltinType.Types.void],
                                 [FakeArgument(attr.type, attr)]),
                                callbackSetterName(attr),
                                descriptor,
                                needThisHandling=False,
                                rethrowContentException=descriptor.interface.isJSImplemented())

    def getRvalDecl(self):
        # We don't need an rval
        return ""

    def getCall(self):
        replacements = {
            "attrName": self.attrName,
            "argv": "argv.handleAt(0)",
            }
        return string.Template(
            'MOZ_ASSERT(argv.length() == 1);\n'
            'if (!JS_SetProperty(cx, mCallback, "${attrName}", ${argv})) {\n'
            '  return Err(FailureUnknown);\n'
            '}\n').substitute(replacements)

    def getArgcDecl(self):
        return None
