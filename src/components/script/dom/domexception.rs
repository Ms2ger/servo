/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::DOMExceptionBinding;
use dom::bindings::utils::{Reflectable, Reflector, reflect_dom_object};
use dom::window::Window;
use servo_util::str::DOMString;

#[repr(uint)]
#[deriving(ToStr)]
enum DOMErrorName {
    IndexSizeError = 1,
    HierarchyRequestError = 3,
    WrongDocumentError = 4,
    InvalidCharacterError = 5,
    NoModificationAllowedError = 7,
    NotFoundError = 8,
    NotSupportedError = 9,
    InvalidStateError = 11,
    SyntaxError = 12,
    InvalidModificationError = 13,
    NamespaceError = 14,
    InvalidAccessError = 15,
    SecurityError = 18,
    NetworkError = 19,
    AbortError = 20,
    URLMismatchError = 21,
    QuotaExceededError = 22,
    TimeoutError = 23,
    InvalidNodeTypeError = 24,
    DataCloneError = 25,
    EncodingError
}

pub struct DOMException {
    code: DOMErrorName,
    reflector_: Reflector
}

impl DOMException {
    pub fn new_inherited(code: DOMErrorName) -> DOMException {
        DOMException {
            code: code,
            reflector_: Reflector::new()
        }
    }

    pub fn new(window: &Window, code: DOMErrorName) -> @mut DOMException {
        reflect_dom_object(@mut DOMException::new_inherited(code), window, DOMExceptionBinding::Wrap)
    }
}

impl Reflectable for DOMException {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        &self.reflector_
    }

    fn mut_reflector<'a>(&'a mut self) -> &'a mut Reflector {
        &mut self.reflector_
    }
}

impl DOMException {
    // http://dom.spec.whatwg.org/#dom-domexception-code
    pub fn Code(&self) -> u16 {
        match self.code {
            // http://dom.spec.whatwg.org/#concept-throw
            EncodingError => 0,
            _ => self.code as u16
        }
    }

    // http://dom.spec.whatwg.org/#error-names-0
    pub fn Name(&self) -> DOMString {
        DOMString::from_string(self.code.to_str())
    }

    // http://dom.spec.whatwg.org/#error-names-0
    pub fn Message(&self) -> DOMString {
        match self.code {
            IndexSizeError => DOMString::from_string("The index is not in the allowed range."),
            HierarchyRequestError => DOMString::from_string("The operation would yield an incorrect node tree."),
            WrongDocumentError => DOMString::from_string("The object is in the wrong document."),
            InvalidCharacterError => DOMString::from_string("The string contains invalid characters."),
            NoModificationAllowedError => DOMString::from_string("The object can not be modified."),
            NotFoundError => DOMString::from_string("The object can not be found here."),
            NotSupportedError => DOMString::from_string("The operation is not supported."),
            InvalidStateError => DOMString::from_string("The object is in an invalid state."),
            SyntaxError => DOMString::from_string("The string did not match the expected pattern."),
            InvalidModificationError => DOMString::from_string("The object can not be modified in this way."),
            NamespaceError => DOMString::from_string("The operation is not allowed by Namespaces in XML."),
            InvalidAccessError => DOMString::from_string("The object does not support the operation or argument."),
            SecurityError => DOMString::from_string("The operation is insecure."),
            NetworkError => DOMString::from_string("A network error occurred."),
            AbortError => DOMString::from_string("The operation was aborted."),
            URLMismatchError => DOMString::from_string("The given URL does not match another URL."),
            QuotaExceededError => DOMString::from_string("The quota has been exceeded."),
            TimeoutError => DOMString::from_string("The operation timed out."),
            InvalidNodeTypeError => DOMString::from_string("The supplied node is incorrect or has an incorrect ancestor for this operation."),
            DataCloneError => DOMString::from_string("The object can not be cloned."),
            EncodingError => DOMString::from_string("The encoding operation (either encoded or decoding) failed."),
        }
    }
}
