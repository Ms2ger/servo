/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::HTMLLinkElementBinding;
use dom::bindings::utils::ErrorResult;
use dom::document::AbstractDocument;
use dom::element::HTMLLinkElementTypeId;
use dom::htmlelement::HTMLElement;
use dom::node::{AbstractNode, Node};
use servo_util::str::DOMString;

pub struct HTMLLinkElement {
    htmlelement: HTMLElement,
}

impl HTMLLinkElement {
    pub fn new_inherited(localName: DOMString, document: AbstractDocument) -> HTMLLinkElement {
        HTMLLinkElement {
            htmlelement: HTMLElement::new_inherited(HTMLLinkElementTypeId, localName, document)
        }
    }

    pub fn new(localName: DOMString, document: AbstractDocument) -> AbstractNode {
        let element = HTMLLinkElement::new_inherited(localName, document);
        Node::reflect_node(@mut element, document, HTMLLinkElementBinding::Wrap)
    }
}

impl HTMLLinkElement {
    pub fn Disabled(&self) -> bool {
        false
    }

    pub fn SetDisabled(&mut self, _disable: bool) {
    }

    pub fn Href(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetHref(&mut self, _href: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn CrossOrigin(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetCrossOrigin(&mut self, _cross_origin: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Rel(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetRel(&mut self, _rel: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Media(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetMedia(&mut self, _media: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Hreflang(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetHreflang(&mut self, _href: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Type(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetType(&mut self, _type: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Charset(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetCharset(&mut self, _charset: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Rev(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetRev(&mut self, _rev: DOMString) -> ErrorResult {
        Ok(())
    }

    pub fn Target(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn SetTarget(&mut self, _target: DOMString) -> ErrorResult {
        Ok(())
    }
}
