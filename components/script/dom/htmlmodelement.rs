/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::HTMLModElementBinding;
use dom::bindings::codegen::InheritTypes::HTMLModElementDerived;
use dom::bindings::js::{JSRef, Temporary};
use dom::bindings::utils::{Reflectable, Reflector};
use dom::document::Document;
use dom::element::ElementTypeId;
use dom::eventtarget::{EventTarget, EventTargetTypeId};
use dom::htmlelement::HTMLElement;
use dom::node::{Node, NodeTypeId};
use servo_util::str::DOMString;

#[dom_struct]
pub struct HTMLModElement {
    htmlelement: HTMLElement
}

impl HTMLModElementDerived for EventTarget {
    fn is_htmlmodelement(&self) -> bool {
        *self.type_id() == EventTargetTypeId::Node(NodeTypeId::Element(ElementTypeId::HTMLModElement))
    }
}

impl HTMLModElement {
    fn new_inherited(localName: DOMString, prefix: Option<DOMString>, document: JSRef<Document>) -> HTMLModElement {
        HTMLModElement {
            htmlelement: HTMLElement::new_inherited(ElementTypeId::HTMLModElement, localName, prefix, document)
        }
    }

    #[allow(unrooted_must_root)]
    pub fn new(localName: DOMString, prefix: Option<DOMString>, document: JSRef<Document>) -> Temporary<HTMLModElement> {
        let element = HTMLModElement::new_inherited(localName, prefix, document);
        Node::reflect_node(box element, document, HTMLModElementBinding::Wrap)
    }
}

impl Reflectable for HTMLModElement {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.htmlelement.reflector()
    }
}
