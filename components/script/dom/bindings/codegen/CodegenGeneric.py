# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import re


class CGThing():
    """
    Abstract base class for things that spit out code.
    """
    def __init__(self):
        pass # Nothing for now

    def define(self):
        """Produce code for a Rust file."""
        assert(False) # Override me!


class CGList(CGThing):
    """
    Generate code for a list of GCThings.  Just concatenates them together, with
    an optional joiner string.  "\n" is a common joiner.
    """
    def __init__(self, children, joiner=""):
        CGThing.__init__(self)
        self.children = children
        self.joiner = joiner

    def append(self, child):
        self.children.append(child)

    def prepend(self, child):
        self.children.insert(0, child)

    def join(self, generator):
        return self.joiner.join(filter(lambda s: len(s) > 0, (child for child in generator)))

    def define(self):
        return self.join(child.define() for child in self.children if child is not None)


class CGIfElseWrapper(CGList):
    def __init__(self, condition, ifTrue, ifFalse):
        kids = [ CGIfWrapper(ifTrue, condition),
                 CGWrapper(CGIndenter(ifFalse), pre=" else {\n", post="\n}") ]
        CGList.__init__(self, kids)


class CGGeneric(CGThing):
    """
    A class that spits out a fixed string into the codegen.  Can spit out a
    separate string for the declaration too.
    """
    def __init__(self, text):
        self.text = text

    def define(self):
        return self.text


class CGWrapper(CGThing):
    """
    Generic CGThing that wraps other CGThings with pre and post text.
    """
    def __init__(self, child, pre="", post="", reindent=False):
        CGThing.__init__(self)
        self.child = child
        self.pre = pre
        self.post = post
        self.reindent = reindent

    def define(self):
        defn = self.child.define()
        if self.reindent:
            # We don't use lineStartDetector because we don't want to
            # insert whitespace at the beginning of our _first_ line.
            defn = stripTrailingWhitespace(
                defn.replace("\n", "\n" + (" " * len(self.pre))))
        return self.pre + defn + self.post


class CGIfWrapper(CGWrapper):
    def __init__(self, child, condition):
        pre = CGWrapper(CGGeneric(condition), pre="if ", post=" {\n",
                        reindent=True)
        CGWrapper.__init__(self, CGIndenter(child), pre=pre.define(),
                           post="\n}")


class CGTemplatedType(CGWrapper):
    def __init__(self, templateName, child):
        CGWrapper.__init__(self, child, pre=templateName + "<", post=">")


class CGNamespace(CGWrapper):
    def __init__(self, namespace, child, public=False):
        pre = "%smod %s {\n" % ("pub " if public else "", namespace)
        post = "} // mod %s\n" % namespace
        CGWrapper.__init__(self, child, pre=pre, post=post)

    @staticmethod
    def build(namespaces, child, public=False):
        """
        Static helper method to build multiple wrapped namespaces.
        """
        if not namespaces:
            return child
        inner = CGNamespace.build(namespaces[1:], child, public=public)
        return CGNamespace(namespaces[0], inner, public=public)


# We'll want to insert the indent at the beginnings of lines, but we
# don't want to indent empty lines.  So only indent lines that have a
# non-newline character on them.
lineStartDetector = re.compile("^(?=[^\n])", re.MULTILINE)
class CGIndenter(CGThing):
    """
    A class that takes another CGThing and generates code that indents that
    CGThing by some number of spaces.  The default indent is two spaces.
    """
    def __init__(self, child, indentLevel=2):
        CGThing.__init__(self)
        self.child = child
        self.indent = " " * indentLevel

    def define(self):
        defn = self.child.define()
        if defn is not "":
            return re.sub(lineStartDetector, self.indent, defn)
        else:
            return defn


class Argument():
    """
    A class for outputting the type and name of an argument
    """
    def __init__(self, argType, name, default=None, mutable=False):
        self.argType = argType
        self.name = name
        self.default = default
        self.mutable = mutable

    def declare(self):
        string = ('mut ' if self.mutable else '') + self.name + ((': ' + self.argType) if self.argType else '')
        #XXXjdm Support default arguments somehow :/
        #if self.default is not None:
        #    string += " = " + self.default
        return string

    def define(self):
        return self.argType + ' ' + self.name


class CGAbstractMethod(CGThing):
    """
    An abstract class for generating code for a method.  Subclasses
    should override definition_body to create the actual code.

    descriptor is the descriptor for the interface the method is associated with

    name is the name of the method as a string

    returnType is the IDLType of the return value

    args is a list of Argument objects

    inline should be True to generate an inline method, whose body is
    part of the declaration.

    alwaysInline should be True to generate an inline method annotated with
    MOZ_ALWAYS_INLINE.

    If templateArgs is not None it should be a list of strings containing
    template arguments, and the function will be templatized using those
    arguments.
    """
    def __init__(self, descriptor, name, returnType, args, inline=False, alwaysInline=False, extern=False, pub=False, templateArgs=None, unsafe=True):
        CGThing.__init__(self)
        self.descriptor = descriptor
        self.name = name
        self.returnType = returnType
        self.args = args
        self.alwaysInline = alwaysInline
        self.extern = extern
        self.templateArgs = templateArgs
        self.pub = pub;
        self.unsafe = unsafe

    def _argstring(self):
        return ', '.join([a.declare() for a in self.args])

    def _template(self):
        if self.templateArgs is None:
            return ''
        return '<%s>\n' % ', '.join(self.templateArgs)

    def _decorators(self):
        decorators = []
        if self.alwaysInline:
            decorators.append('#[inline(always)]')

        if self.extern:
            decorators.append('unsafe')
            decorators.append('extern')

        if self.pub:
            decorators.append('pub')

        if not decorators:
            return ''
        return ' '.join(decorators) + ' '

    def _returnType(self):
        return (" -> %s" % self.returnType) if self.returnType != "void" else ""

    def define(self):
        body = self.definition_body()
        if self.unsafe:
            body = CGWrapper(body, pre="unsafe {\n", post="\n}")

        return CGWrapper(CGIndenter(body),
                         pre=self.definition_prologue(),
                         post=self.definition_epilogue()).define()

    def definition_prologue(self):
        return "%sfn %s%s(%s)%s {\n" % (self._decorators(), self.name, self._template(),
                                          self._argstring(), self._returnType())

    def definition_epilogue(self):
        return "\n}\n"

    def definition_body(self):
        assert(False) # Override me!


class CGAbstractExternMethod(CGAbstractMethod):
    """
    Abstract base class for codegen of implementation-only (no
    declaration) static methods.
    """
    def __init__(self, descriptor, name, returnType, args):
        CGAbstractMethod.__init__(self, descriptor, name, returnType, args,
                                  inline=False, extern=True)


def toStringBool(arg):
    return str(not not arg).lower()


def stripTrailingWhitespace(text):
    tail = '\n' if text.endswith('\n') else ''
    lines = text.splitlines()
    for i in range(len(lines)):
        lines[i] = lines[i].rstrip()
    return '\n'.join(lines) + tail
