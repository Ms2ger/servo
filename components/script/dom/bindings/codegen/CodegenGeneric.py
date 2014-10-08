# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import re
import string


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
        s = ('mut ' if self.mutable else '') + self.name + ((': ' + self.argType) if self.argType else '')
        #XXXjdm Support default arguments somehow :/
        #if self.default is not None:
        #    string += " = " + self.default
        return s

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


class ClassItem:
    """ Use with CGClass """
    def __init__(self, name, visibility):
        self.name = name
        self.visibility = visibility
    def declare(self, cgClass):
        assert False
    def define(self, cgClass):
        assert False

class ClassBase(ClassItem):
    def __init__(self, name, visibility='pub'):
        ClassItem.__init__(self, name, visibility)
    def declare(self, cgClass):
        return '%s %s' % (self.visibility, self.name)
    def define(self, cgClass):
        # Only in the header
        return ''

class ClassMethod(ClassItem):
    def __init__(self, name, returnType, args, inline=False, static=False,
                 virtual=False, const=False, bodyInHeader=False,
                 templateArgs=None, visibility='public', body=None,
                 breakAfterReturnDecl="\n",
                 breakAfterSelf="\n", override=False):
        """
        override indicates whether to flag the method as MOZ_OVERRIDE
        """
        assert not override or virtual
        self.returnType = returnType
        self.args = args
        self.inline = False
        self.static = static
        self.virtual = virtual
        self.const = const
        self.bodyInHeader = True
        self.templateArgs = templateArgs
        self.body = body
        self.breakAfterReturnDecl = breakAfterReturnDecl
        self.breakAfterSelf = breakAfterSelf
        self.override = override
        ClassItem.__init__(self, name, visibility)

    def getDecorators(self, declaring):
        decorators = []
        if self.inline:
            decorators.append('inline')
        if declaring:
            if self.static:
                decorators.append('static')
            if self.virtual:
                decorators.append('virtual')
        if decorators:
            return ' '.join(decorators) + ' '
        return ''

    def getBody(self):
        # Override me or pass a string to constructor
        assert self.body is not None
        return self.body

    def declare(self, cgClass):
        templateClause = '<%s>' % ', '.join(self.templateArgs) \
                         if self.bodyInHeader and self.templateArgs else ''
        args = ', '.join([a.declare() for a in self.args])
        if self.bodyInHeader:
            body = CGIndenter(CGGeneric(self.getBody())).define()
            body = ' {\n' + body + '\n}'
        else:
           body = ';'

        return string.Template("${decorators}%s"
                               "${visibility}fn ${name}${templateClause}(${args})${returnType}${const}${override}${body}%s" %
                               (self.breakAfterReturnDecl, self.breakAfterSelf)
                               ).substitute({
                'templateClause': templateClause,
                'decorators': self.getDecorators(True),
                'returnType': (" -> %s" % self.returnType) if self.returnType else "",
                'name': self.name,
                'const': ' const' if self.const else '',
                'override': ' MOZ_OVERRIDE' if self.override else '',
                'args': args,
                'body': body,
                'visibility': self.visibility + ' ' if self.visibility is not 'priv' else ''
                })

    def define(self, cgClass):
        pass

class ClassUsingDeclaration(ClassItem):
    """"
    Used for importing a name from a base class into a CGClass

    baseClass is the name of the base class to import the name from

    name is the name to import

    visibility determines the visibility of the name (public,
    protected, private), defaults to public.
    """
    def __init__(self, baseClass, name, visibility='public'):
        self.baseClass = baseClass
        ClassItem.__init__(self, name, visibility)

    def declare(self, cgClass):
        return string.Template("""using ${baseClass}::${name};
""").substitute({ 'baseClass': self.baseClass,
                  'name': self.name })

    def define(self, cgClass):
        return ''

class ClassConstructor(ClassItem):
    """
    Used for adding a constructor to a CGClass.

    args is a list of Argument objects that are the arguments taken by the
    constructor.

    inline should be True if the constructor should be marked inline.

    bodyInHeader should be True if the body should be placed in the class
    declaration in the header.

    visibility determines the visibility of the constructor (public,
    protected, private), defaults to private.

    explicit should be True if the constructor should be marked explicit.

    baseConstructors is a list of strings containing calls to base constructors,
    defaults to None.

    body contains a string with the code for the constructor, defaults to empty.
    """
    def __init__(self, args, inline=False, bodyInHeader=False,
                 visibility="priv", explicit=False, baseConstructors=None,
                 body=""):
        self.args = args
        self.inline = False
        self.bodyInHeader = bodyInHeader
        self.explicit = explicit
        self.baseConstructors = baseConstructors or []
        self.body = body
        ClassItem.__init__(self, None, visibility)

    def getDecorators(self, declaring):
        decorators = []
        if self.explicit:
            decorators.append('explicit')
        if self.inline and declaring:
            decorators.append('inline')
        if decorators:
            return ' '.join(decorators) + ' '
        return ''

    def getInitializationList(self, cgClass):
        items = [str(c) for c in self.baseConstructors]
        for m in cgClass.members:
            if not m.static:
                initialize = m.body
                if initialize:
                    items.append(m.name + "(" + initialize + ")")

        if len(items) > 0:
            return '\n  : ' + ',\n    '.join(items)
        return ''

    def getBody(self, cgClass):
        initializers = ["  parent: %s" % str(self.baseConstructors[0])]
        return (self.body + (
                "%s {\n"
                "%s\n"
                "}") % (cgClass.name, '\n'.join(initializers)))

    def declare(self, cgClass):
        args = ', '.join([a.declare() for a in self.args])
        body = '  ' + self.getBody(cgClass);
        body = stripTrailingWhitespace(body.replace('\n', '\n  '))
        if len(body) > 0:
            body += '\n'
        body = ' {\n' + body + '}'

        return string.Template("""pub fn ${decorators}new(${args}) -> ${className}${body}
""").substitute({ 'decorators': self.getDecorators(True),
                  'className': cgClass.getNameString(),
                  'args': args,
                  'body': body })

    def define(self, cgClass):
        if self.bodyInHeader:
            return ''

        args = ', '.join([a.define() for a in self.args])

        body = '  ' + self.getBody()
        body = '\n' + stripTrailingWhitespace(body.replace('\n', '\n  '))
        if len(body) > 0:
            body += '\n'

        return string.Template("""${decorators}
${className}::${className}(${args})${initializationList}
{${body}}
""").substitute({ 'decorators': self.getDecorators(False),
                  'className': cgClass.getNameString(),
                  'args': args,
                  'initializationList': self.getInitializationList(cgClass),
                  'body': body })

class ClassDestructor(ClassItem):
    """
    Used for adding a destructor to a CGClass.

    inline should be True if the destructor should be marked inline.

    bodyInHeader should be True if the body should be placed in the class
    declaration in the header.

    visibility determines the visibility of the destructor (public,
    protected, private), defaults to private.

    body contains a string with the code for the destructor, defaults to empty.

    virtual determines whether the destructor is virtual, defaults to False.
    """
    def __init__(self, inline=False, bodyInHeader=False,
                 visibility="private", body='', virtual=False):
        self.inline = inline or bodyInHeader
        self.bodyInHeader = bodyInHeader
        self.body = body
        self.virtual = virtual
        ClassItem.__init__(self, None, visibility)

    def getDecorators(self, declaring):
        decorators = []
        if self.virtual and declaring:
            decorators.append('virtual')
        if self.inline and declaring:
            decorators.append('inline')
        if decorators:
            return ' '.join(decorators) + ' '
        return ''

    def getBody(self):
        return self.body

    def declare(self, cgClass):
        if self.bodyInHeader:
            body = '  ' + self.getBody();
            body = stripTrailingWhitespace(body.replace('\n', '\n  '))
            if len(body) > 0:
                body += '\n'
            body = '\n{\n' + body + '}'
        else:
            body = ';'

        return string.Template("""${decorators}~${className}()${body}
""").substitute({ 'decorators': self.getDecorators(True),
                  'className': cgClass.getNameString(),
                  'body': body })

    def define(self, cgClass):
        if self.bodyInHeader:
            return ''

        body = '  ' + self.getBody()
        body = '\n' + stripTrailingWhitespace(body.replace('\n', '\n  '))
        if len(body) > 0:
            body += '\n'

        return string.Template("""${decorators}
${className}::~${className}()
{${body}}
""").substitute({ 'decorators': self.getDecorators(False),
                  'className': cgClass.getNameString(),
                  'body': body })

class ClassMember(ClassItem):
    def __init__(self, name, type, visibility="priv", static=False,
                 body=None):
        self.type = type;
        self.static = static
        self.body = body
        ClassItem.__init__(self, name, visibility)

    def declare(self, cgClass):
        return '%s %s: %s,\n' % (self.visibility, self.name, self.type)

    def define(self, cgClass):
        if not self.static:
            return ''
        if self.body:
            body = " = " + self.body
        else:
            body = ""
        return '%s %s::%s%s;\n' % (self.type, cgClass.getNameString(),
                                      self.name, body)

class ClassTypedef(ClassItem):
    def __init__(self, name, type, visibility="public"):
        self.type = type
        ClassItem.__init__(self, name, visibility)

    def declare(self, cgClass):
        return 'typedef %s %s;\n' % (self.type, self.name)

    def define(self, cgClass):
        # Only goes in the header
        return ''

class ClassEnum(ClassItem):
    def __init__(self, name, entries, values=None, visibility="public"):
        self.entries = entries
        self.values = values
        ClassItem.__init__(self, name, visibility)

    def declare(self, cgClass):
        entries = []
        for i in range(0, len(self.entries)):
            if not self.values or i >= len(self.values):
                entry = '%s' % self.entries[i]
            else:
                entry = '%s = %s' % (self.entries[i], self.values[i])
            entries.append(entry)
        name = '' if not self.name else ' ' + self.name
        return 'enum%s\n{\n  %s\n};\n' % (name, ',\n  '.join(entries))

    def define(self, cgClass):
        # Only goes in the header
        return ''

class ClassUnion(ClassItem):
    def __init__(self, name, entries, visibility="public"):
        self.entries = [entry + ";" for entry in entries]
        ClassItem.__init__(self, name, visibility)

    def declare(self, cgClass):
        return 'union %s\n{\n  %s\n};\n' % (self.name, '\n  '.join(self.entries))

    def define(self, cgClass):
        # Only goes in the header
        return ''

class CGClass(CGThing):
    def __init__(self, name, bases=[], members=[], constructors=[],
                 destructor=None, methods=[],
                 typedefs = [], enums=[], unions=[], templateArgs=[],
                 templateSpecialization=[], isStruct=False,
                 disallowCopyConstruction=False, indent='',
                 decorators='',
                 extradeclarations='',
                 extradefinitions=''):
        CGThing.__init__(self)
        self.name = name
        self.bases = bases
        self.members = members
        self.constructors = constructors
        # We store our single destructor in a list, since all of our
        # code wants lists of members.
        self.destructors = [destructor] if destructor else []
        self.methods = methods
        self.typedefs = typedefs
        self.enums = enums
        self.unions = unions
        self.templateArgs = templateArgs
        self.templateSpecialization = templateSpecialization
        self.isStruct = isStruct
        self.disallowCopyConstruction = disallowCopyConstruction
        self.indent = indent
        self.decorators = decorators
        self.extradeclarations = extradeclarations
        self.extradefinitions = extradefinitions

    def getNameString(self):
        className = self.name
        if self.templateSpecialization:
            className = className + \
                '<%s>' % ', '.join([str(a) for a
                                    in self.templateSpecialization])
        return className

    def define(self):
        result = ''
        if self.templateArgs:
            templateArgs = [a.declare() for a in self.templateArgs]
            templateArgs = templateArgs[len(self.templateSpecialization):]
            result = result + self.indent + 'template <%s>\n' \
                     % ','.join([str(a) for a in templateArgs])

        if self.templateSpecialization:
            specialization = \
                '<%s>' % ', '.join([str(a) for a in self.templateSpecialization])
        else:
            specialization = ''

        myself = ''
        if self.decorators != '':
            myself += self.decorators + '\n'
        myself += '%spub struct %s%s' % (self.indent, self.name, specialization)
        result += myself

        assert len(self.bases) == 1 #XXjdm Can we support multiple inheritance?

        result += '{\n%s\n' % self.indent

        if self.bases:
            self.members = [ClassMember("parent", self.bases[0].name, "pub")] + self.members

        result += CGIndenter(CGGeneric(self.extradeclarations),
                             len(self.indent)).define()

        def declareMembers(cgClass, memberList):
            result = ''

            for member in memberList:
                declaration = member.declare(cgClass)
                declaration = CGIndenter(CGGeneric(declaration)).define()
                result = result + declaration
            return result

        if self.disallowCopyConstruction:
            class DisallowedCopyConstructor(object):
                def __init__(self):
                    self.visibility = "private"
                def declare(self, cgClass):
                    name = cgClass.getNameString()
                    return ("%s(const %s&) MOZ_DELETE;\n"
                            "void operator=(const %s) MOZ_DELETE;\n" % (name, name, name))
            disallowedCopyConstructors = [DisallowedCopyConstructor()]
        else:
            disallowedCopyConstructors = []

        order = [(self.enums, ''), (self.unions, ''),
                 (self.typedefs, ''), (self.members, '')]

        for (memberList, separator) in order:
            memberString = declareMembers(self, memberList)
            if self.indent:
                memberString = CGIndenter(CGGeneric(memberString),
                                          len(self.indent)).define()
            result = result + memberString

        result += self.indent + '}\n\n'
        result += 'impl %s {\n' % self.name

        order = [(self.constructors + disallowedCopyConstructors, '\n'),
                 (self.destructors, '\n'), (self.methods, '\n)')]
        for (memberList, separator) in order:
            memberString = declareMembers(self, memberList)
            if self.indent:
                memberString = CGIndenter(CGGeneric(memberString),
                                          len(self.indent)).define()
            result = result + memberString

        result += "}"
        return result


def toStringBool(arg):
    return str(not not arg).lower()


def stripTrailingWhitespace(text):
    tail = '\n' if text.endswith('\n') else ''
    lines = text.splitlines()
    for i in range(len(lines)):
        lines[i] = lines[i].rstrip()
    return '\n'.join(lines) + tail
