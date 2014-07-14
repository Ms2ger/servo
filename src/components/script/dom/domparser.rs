/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::DOMParserBinding;
use dom::bindings::codegen::Bindings::DOMParserBinding::SupportedTypeValues::{Text_html, Text_xml};
use dom::bindings::error::{Fallible, FailureUnknown};
use dom::bindings::global::{GlobalRef, Window};
use dom::bindings::js::{JS, JSRef, Temporary};
use dom::bindings::utils::{Reflector, Reflectable, reflect_dom_object};
use dom::document::{Document, HTMLDocument, NonHTMLDocument};
use dom::window::Window;
use servo_util::str::DOMString;

#[deriving(Encodable)]
pub struct DOMParser {
    pub owner: JS<Window>, //XXXjdm Document instead?
    pub reflector_: Reflector
}

impl DOMParser {
    pub fn new_inherited(owner: &JSRef<Window>) -> DOMParser {
        DOMParser {
            owner: JS::from_rooted(owner),
            reflector_: Reflector::new()
        }
    }

    pub fn new(owner: &JSRef<Window>) -> Temporary<DOMParser> {
        reflect_dom_object(box DOMParser::new_inherited(owner), &Window(owner),
                           DOMParserBinding::Wrap)
    }

    pub fn Constructor(global: &GlobalRef) -> Fallible<Temporary<DOMParser>> {
        Ok(DOMParser::new(global.as_window()))
    }
}

pub trait DOMParserMethods {
    fn ParseFromString(&self, _s: DOMString, ty: DOMParserBinding::SupportedType)
        -> Fallible<Temporary<Document>>;
}

impl<'a> DOMParserMethods for JSRef<'a, DOMParser> {
    fn ParseFromString(&self,
                       _s: DOMString,
                       ty: DOMParserBinding::SupportedType)
                       -> Fallible<Temporary<Document>> {
        let owner = self.owner.root();
        match ty {
            Text_html => {
                Ok(Document::new(&owner.root_ref(), None, HTMLDocument, Some("text/html".to_string())))
            }
            Text_xml => {
                Ok(Document::new(&owner.root_ref(), None, NonHTMLDocument, Some("text/xml".to_string())))
            }
            _ => {
                Err(FailureUnknown)
            }
        }
    }
}

impl Reflectable for DOMParser {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        &self.reflector_
    }
}
