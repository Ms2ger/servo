/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::HTMLAppletElementBinding;
use dom::bindings::utils::ErrorResult;
use dom::document::AbstractDocument;
use dom::element::HTMLAppletElementTypeId;
use dom::htmlelement::HTMLElement;
use dom::node::{AbstractNode, Node};
use servo_util::str::DOMString;

pub struct HTMLAppletElement {
    htmlelement: HTMLElement
}

impl HTMLAppletElement {
    pub fn new_inherited(localName: DOMString, document: AbstractDocument) -> HTMLAppletElement {
        HTMLAppletElement {
            htmlelement: HTMLElement::new_inherited(HTMLAppletElementTypeId, localName, document)
        }
    }

    pub fn new(localName: DOMString, document: AbstractDocument) -> AbstractNode {
        let element = HTMLAppletElement::new_inherited(localName, document);
        Node::reflect_node(@mut element, document, HTMLAppletElementBinding::Wrap)
    }
}

impl HTMLAppletElement {
    pub fn Align(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetAlign(&mut self, _align: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Alt(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetAlt(&self, _alt: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Archive(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetArchive(&self, _archive: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Code(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetCode(&self, _code: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn CodeBase(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetCodeBase(&self, _code_base: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Height(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetHeight(&self, _height: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Hspace(&self) -> u32 {
        0
    }

    pub fn SetHspace(&mut self, _hspace: u32) -> ErrorResult {
        Ok(())
    }

    pub fn Name(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetName(&mut self, _name: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Object(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetObject(&mut self, _object: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Vspace(&self) -> u32 {
        0
    }

    pub fn SetVspace(&mut self, _vspace: u32) -> ErrorResult {
        Ok(())
    }

    pub fn Width(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetWidth(&mut self, _width: DOMString) -> ErrorResult {
        Ok(())
    }
}
