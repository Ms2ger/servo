/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::CSSStyleDeclarationBinding;
use dom::bindings::global::Window;
use dom::bindings::js::{JSRef, Temporary};
use dom::bindings::utils::{Reflector, Reflectable, reflect_dom_object};
use dom::window::Window;

#[deriving(Encodable)]
pub struct CSSStyleDeclaration {
    reflector_: Reflector,
}

impl CSSStyleDeclaration {
    pub fn new_inherited() -> CSSStyleDeclaration {
        CSSStyleDeclaration {
            reflector_: Reflector::new(),
        }
    }

    pub fn new(window: &JSRef<Window>) -> Temporary<CSSStyleDeclaration> {
        reflect_dom_object(box CSSStyleDeclaration::new_inherited(),
                           &Window(*window), CSSStyleDeclarationBinding::Wrap)
    }
}

impl Reflectable for CSSStyleDeclaration {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        &self.reflector_
    }
}

pub trait CSSStyleDeclarationMethods {
    fn GetPropertyValue(&self);
}

impl<'a> CSSStyleDeclarationMethods for JSRef<'a, CSSStyleDeclaration> {
    fn GetPropertyValue(&self) {}
}
