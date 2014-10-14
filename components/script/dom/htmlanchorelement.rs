/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::AttrBinding::AttrMethods;
use dom::bindings::codegen::Bindings::EventBinding::EventMethods;
use dom::bindings::codegen::Bindings::HTMLAnchorElementBinding;
use dom::bindings::codegen::Bindings::HTMLAnchorElementBinding::HTMLAnchorElementMethods;
use dom::bindings::codegen::Bindings::NodeBinding::NodeMethods;
use dom::bindings::codegen::InheritTypes::HTMLAnchorElementDerived;
use dom::bindings::codegen::InheritTypes::{ElementCast, HTMLElementCast, NodeCast};
use dom::bindings::js::{JSRef, Temporary, OptionalRootable};
use dom::bindings::utils::{Reflectable, Reflector};
use dom::document::{Document, DocumentHelpers};
use dom::element::{Element, AttributeHandlers, HTMLAnchorElementTypeId};
use dom::event::Event;
use dom::eventtarget::{EventTarget, NodeTargetTypeId};
use dom::htmlelement::HTMLElement;
use dom::node::{Node, NodeHelpers, ElementNodeTypeId};
use dom::virtualmethods::VirtualMethods;

use servo_util::str::DOMString;

#[jstraceable]
#[must_root]
#[privatize]
pub struct HTMLAnchorElement {
    htmlelement: HTMLElement
}

impl HTMLAnchorElementDerived for EventTarget {
    fn is_htmlanchorelement(&self) -> bool {
        *self.type_id() == NodeTargetTypeId(ElementNodeTypeId(HTMLAnchorElementTypeId))
    }
}

impl HTMLAnchorElement {
    fn new_inherited(localName: DOMString, prefix: Option<DOMString>, document: JSRef<Document>) -> HTMLAnchorElement {
        HTMLAnchorElement {
            htmlelement: HTMLElement::new_inherited(HTMLAnchorElementTypeId, localName, prefix, document)
        }
    }

    #[allow(unrooted_must_root)]
    pub fn new(localName: DOMString, prefix: Option<DOMString>, document: JSRef<Document>) -> Temporary<HTMLAnchorElement> {
        let element = HTMLAnchorElement::new_inherited(localName, prefix, document);
        Node::reflect_node(box element, document, HTMLAnchorElementBinding::Wrap)
    }
}

trait PrivateHTMLAnchorElementHelpers {
    fn handle_event_impl(self, event: JSRef<Event>);
}

impl<'a> PrivateHTMLAnchorElementHelpers for JSRef<'a, HTMLAnchorElement> {
    fn handle_event_impl(self, event: JSRef<Event>) {
        let element: JSRef<Element> = ElementCast::from_ref(self);
        let attr = element.get_attribute(ns!(""), "href").root();
        match attr {
            Some(ref href) => {
                let value = href.Value();
                debug!("clicked on link to {:s}", value);
                let node: JSRef<Node> = NodeCast::from_ref(self);
                let doc = node.owner_doc().root();
                doc.load_anchor_href(value);
            }
            None => ()
        }
    }

    fn follow_the_hyperlink(self) {
        // Step 1.
        let replace = false.

        // 2. Let source be the browsing context that contains the Document
        //    object with which subject in question is associated.

        // 3. If the user indicated a specific browsing context when following
        //    the hyperlink, or if the user agent is configured to follow
        //    hyperlinks by navigating a particular browsing context, then let
        //    target be that browsing context. If this is a new top-level
        //    browsing context (e.g. when the user followed the hyperlink
        //    using "Open in New Tab"), then source must be set as the new
        //    browsing context's one permitted sandboxed navigator.
        //
        //    Otherwise, if subject is an a or area element that has a target
        //    attribute, then let target be the browsing context that is chosen
        //    by applying the rules for choosing a browsing context given a
        //    browsing context name, using the value of the target attribute as
        //    the browsing context name. If these rules result in the creation
        //    of a new browsing context, set replace to true.
        //
        //    Otherwise, if the hyperlink is a sidebar hyperlink, the user
        //    agent implements a feature that can be considered a secondary
        //    browsing context, and the user agent intends to use this feature
        //    in this instance, let target be such a secondary browsing
        //    context.
        //
        //    Otherwise, if target is an a or area element with no target
        //    attribute, but the Document contains a base element with a target
        //    attribute, then let target be the browsing context that is chosen
        //    by applying the rules for choosing a browsing context given a
        //    browsing context name, using the value of the target attribute of
        //    the first such base element as the browsing context name. If
        //    these rules result in the creation of a new browsing context, set
        //    replace to true.

        let target = self.owner_doc().browsing_context();

Resolve the URL given by the href attribute of that element, relative to that element.

If that is successful, let URL be the resulting absolute URL.

Otherwise, if resolving the URL failed, the user agent may report the error to the user in a user-agent-specific manner, may queue a task to navigate the target browsing context to an error page to report the error, or may ignore the error and do nothing. In any case, the user agent must then abort these steps.

In the case of server-side image maps, append the hyperlink suffix to URL.

Queue a task to navigate the target browsing context to URL. If replace is true, the navigation must be performed with replacement enabled. The source browsing context must be source.
    }
}

impl<'a> VirtualMethods for JSRef<'a, HTMLAnchorElement> {
    fn super_type<'a>(&'a self) -> Option<&'a VirtualMethods> {
        let htmlelement: &JSRef<HTMLElement> = HTMLElementCast::from_borrowed_ref(self);
        Some(htmlelement as &VirtualMethods)
    }
}

impl Reflectable for HTMLAnchorElement {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.htmlelement.reflector()
    }
}

impl<'a> HTMLAnchorElementMethods for JSRef<'a, HTMLAnchorElement> {
    fn Text(self) -> DOMString {
        let node: JSRef<Node> = NodeCast::from_ref(self);
        node.GetTextContent().unwrap()
    }

    fn SetText(self, value: DOMString) {
        let node: JSRef<Node> = NodeCast::from_ref(self);
        node.SetTextContent(Some(value))
    }
}

pub fn activation_behavior(this: JSRef<HTMLAnchorElement>) {
    // 1. If the a element's node document is not fully active, then abort
    //    these steps.

    // 2. If either the a element has a download attribute and the algorithm
    //    is not allowed to show a popup; or, if the user has not indicated a
    //    specific browsing context for following the link, and the element's
    //    target attribute is present, and applying the rules for choosing a
    //    browsing context given a browsing context name, using the value of
    //    the target attribute as the browsing context name, would result in
    //    there not being a chosen browsing context, then run these substeps:
    //    1. If there is an entry settings object, throw an InvalidAccessError
    //       exception.
    //    2. Abort these steps without following the hyperlink.

    // 3. If the target of the click event is an img element with an ismap
    //    attribute specified, then server-side image map processing must be
    //    performed, as follows:
    //    1. If the click event was a real pointing-device-triggered click
    //       event on the img element, then let x be the distance in CSS pixels
    //       from the left edge of the image's left border, if it has one, or
    //       the left edge of the image otherwise, to the location of the
    //       click, and let y be the distance in CSS pixels from the top edge
    //       of the image's top border, if it has one, or the top edge of the
    //       image otherwise, to the location of the click. Otherwise, let x
    //       and y be zero.
    //    2. Let the hyperlink suffix be a U+003F QUESTION MARK character, the
    //       value of x expressed as a base-ten integer using ASCII digits, a
    //       U+002C COMMA character (,), and the value of y expressed as a
    //       base-ten integer using ASCII digits.

    // Step 4.
    this.follow_the_hyperlink();
}
