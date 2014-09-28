/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::attr::AttrHelpers;
use dom::bindings::codegen::Bindings::HTMLIFrameElementBinding;
use dom::bindings::codegen::Bindings::HTMLIFrameElementBinding::HTMLIFrameElementMethods;
use dom::bindings::codegen::InheritTypes::{NodeCast, ElementCast};
use dom::bindings::codegen::InheritTypes::{HTMLElementCast, HTMLIFrameElementDerived};
use dom::bindings::js::{JSRef, Temporary, OptionalRootable};
use dom::bindings::trace::Traceable;
use dom::bindings::utils::{Reflectable, Reflector};
use dom::browsercontext::BrowserContext;
use dom::document::Document;
use dom::element::{HTMLIFrameElementTypeId, Element};
use dom::element::AttributeHandlers;
use dom::eventtarget::{EventTarget, NodeTargetTypeId};
use dom::htmlelement::HTMLElement;
use dom::node::{Node, NodeHelpers, ElementNodeTypeId, window_from_node};
use dom::virtualmethods::VirtualMethods;
use dom::window::Window;
use page::IterablePage;

use servo_msg::constellation_msg::{PipelineId, SubpageId};
use servo_msg::constellation_msg::{IFrameSandboxed, IFrameUnsandboxed};
use servo_msg::constellation_msg::{ConstellationChan, LoadIframeUrlMsg};
use servo_util::atom::Atom;
use servo_util::namespace::Null;
use servo_util::str::DOMString;

use std::ascii::StrAsciiExt;
use std::cell::{Cell, RefCell};
use url::{Url, UrlParser};

enum SandboxAllowance {
    AllowNothing = 0x00,
    AllowSameOrigin = 0x01,
    AllowTopNavigation = 0x02,
    AllowForms = 0x04,
    AllowScripts = 0x08,
    AllowPointerLock = 0x10,
    AllowPopups = 0x20
}

#[jstraceable]
#[must_root]
pub struct HTMLIFrameElement {
    pub htmlelement: HTMLElement,
    pub size: Traceable<Cell<Option<IFrameSize>>>,
    pub sandbox: Traceable<Cell<Option<u8>>>,
    browser_context: RefCell<Option<BrowserContext>>,
}

impl HTMLIFrameElementDerived for EventTarget {
    fn is_htmliframeelement(&self) -> bool {
        self.type_id == NodeTargetTypeId(ElementNodeTypeId(HTMLIFrameElementTypeId))
    }
}

#[jstraceable]
pub struct IFrameSize {
    pub pipeline_id: PipelineId,
    pub subpage_id: SubpageId,
}

pub trait HTMLIFrameElementHelpers {
    fn is_sandboxed(self) -> bool;
    fn get_url(self) -> Url;
    /// http://www.whatwg.org/html/#process-the-iframe-attributes
    fn process_the_iframe_attributes(self, first_time: bool);
}

impl<'a> HTMLIFrameElementHelpers for JSRef<'a, HTMLIFrameElement> {
    fn is_sandboxed(self) -> bool {
        self.sandbox.deref().get().is_some()
    }

    fn get_url(self) -> Url {
        // If the value of the src attribute is missing, or its value is the
        // empty string, let url be the string "about:blank".
        //
        // Otherwise, resolve the value of the src attribute, relative to the
        // iframe element.
        //
        // If that is not successful, then let url be the string "about:blank".
        // Otherwise, let url be the resulting absolute URL.

        let element: JSRef<Element> = ElementCast::from_ref(self);
        let url = element.get_attribute(Null, "src").root().and_then(|src| {
            let url_value = src.deref().value();
            let url_string = url_value.as_slice();
            if url_string.is_empty() {
                None
            } else {
                let window = window_from_node(self).root();
                UrlParser::new().base_url(&window.deref().page().get_url())
                    .parse(url_string).ok()
            }
        });

        match url {
            Some(url) => url,
            None => Url::parse("about:blank").unwrap(),
        }
    }

    fn process_the_iframe_attributes(self, first_time: bool) {
        fn navigate(_url: Url) {
            // Any navigation required of the user agent in the process the
            // iframe attributes algorithm must be completed as an explicit
            // self-navigation override and with the iframe element's node
            // document's browsing context as the source browsing context.
            //
            // Furthermore, if the active document of the element's child
            // browsing context before such a navigation was not completely
            // loaded at the time of the new navigation, then the navigation
            // must be completed with replacement enabled.
            //
            // Similarly, if the child browsing context's session history
            // contained only one Document when the process the iframe
            // attributes algorithm was invoked, and that was the about:blank
            // Document created when the child browsing context was created,
            // then any navigation required of the user agent in that
            // algorithm must be completed with replacement enabled.
        }

        assert!(self.browser_context.borrow().is_some());

        // Case 1: srcdoc.
        let element: JSRef<Element> = ElementCast::from_ref(self);
        match element.get_attribute(Null, "src").root() {
            // Case 2.
            None if first_time => {
                // Queue a task to run the iframe load event steps.
                // The task source for this task is the DOM manipulation task source.
            }
            // Case 3.
            attr => {
                // Step 1.
                let url = self.get_url();

                // Step 2.
                // If there exists an ancestor browsing context whose active
                // document's address, ignoring fragment identifiers, is equal
                // to url, then abort these steps.

                // Step 3.
                navigate(url);
            }
        }
/*
        let sandboxed = if self.is_sandboxed() {
            IFrameSandboxed
        } else {
            IFrameUnsandboxed
        };

        // Subpage Id
        let window = window_from_node(self).root();
        let page = window.deref().page();
        let subpage_id = page.get_next_subpage_id();

        self.deref().size.deref().set(Some(IFrameSize {
            pipeline_id: page.id,
            subpage_id: subpage_id,
        }));

        let ConstellationChan(ref chan) = *page.constellation_chan.deref();
        chan.send(LoadIframeUrlMsg(url, page.id, subpage_id, sandboxed));
*/
    }
}

impl HTMLIFrameElement {
    fn new_inherited(localName: DOMString, document: JSRef<Document>) -> HTMLIFrameElement {
        HTMLIFrameElement {
            htmlelement: HTMLElement::new_inherited(HTMLIFrameElementTypeId, localName, document),
            size: Traceable::new(Cell::new(None)),
            sandbox: Traceable::new(Cell::new(None)),
            browser_context: RefCell::new(None),
        }
    }

    #[allow(unrooted_must_root)]
    pub fn new(localName: DOMString, document: JSRef<Document>) -> Temporary<HTMLIFrameElement> {
        let element = HTMLIFrameElement::new_inherited(localName, document);
        Node::reflect_node(box element, document, HTMLIFrameElementBinding::Wrap)
    }
}

impl<'a> HTMLIFrameElementMethods for JSRef<'a, HTMLIFrameElement> {
    fn Src(self) -> DOMString {
        let element: JSRef<Element> = ElementCast::from_ref(self);
        element.get_string_attribute("src")
    }

    fn SetSrc(self, src: DOMString) {
        let element: JSRef<Element> = ElementCast::from_ref(self);
        element.set_url_attribute("src", src)
    }

    fn Sandbox(self) -> DOMString {
        let element: JSRef<Element> = ElementCast::from_ref(self);
        element.get_string_attribute("sandbox")
    }

    fn SetSandbox(self, sandbox: DOMString) {
        let element: JSRef<Element> = ElementCast::from_ref(self);
        element.set_string_attribute("sandbox", sandbox);
    }

    fn GetContentWindow(self) -> Option<Temporary<Window>> {
        self.size.deref().get().and_then(|size| {
            let window = window_from_node(self).root();
            let children = window.deref().page.children.deref().borrow();
            let child = children.iter().find(|child| {
                child.subpage_id.unwrap() == size.subpage_id
            });
            child.and_then(|page| {
                page.frame.deref().borrow().as_ref().map(|frame| {
                    Temporary::new(frame.window.clone())
                })
            })
        })
    }
}

impl<'a> VirtualMethods for JSRef<'a, HTMLIFrameElement> {
    fn super_type<'a>(&'a self) -> Option<&'a VirtualMethods> {
        let htmlelement: &JSRef<HTMLElement> = HTMLElementCast::from_borrowed_ref(self);
        Some(htmlelement as &VirtualMethods)
    }

    fn after_set_attr(&self, name: &Atom, value: DOMString) {
        match self.super_type() {
            Some(ref s) => s.after_set_attr(name, value.clone()),
            _ => (),
        }

        if "sandbox" == name.as_slice() {
            let mut modes = AllowNothing as u8;
            for word in value.as_slice().split(' ') {
                modes |= match word.to_ascii_lower().as_slice() {
                    "allow-same-origin" => AllowSameOrigin,
                    "allow-forms" => AllowForms,
                    "allow-pointer-lock" => AllowPointerLock,
                    "allow-popups" => AllowPopups,
                    "allow-scripts" => AllowScripts,
                    "allow-top-navigation" => AllowTopNavigation,
                    _ => AllowNothing
                } as u8;
            }
            self.deref().sandbox.deref().set(Some(modes));
        }

        if "src" == name.as_slice() {
            // whenever an iframe element with a nested browsing context but
            // with no srcdoc attribute specified has its src attribute set,
            // changed, or removed, the user agent must process the iframe
            // attributes.
            if self.browser_context.borrow().is_some() {
                self.process_the_iframe_attributes(false)
            }
        }
    }

    fn before_remove_attr(&self, name: &Atom, value: DOMString) {
        match self.super_type() {
            Some(ref s) => s.before_remove_attr(name, value),
            _ => (),
        }

        if "sandbox" == name.as_slice() {
            self.deref().sandbox.deref().set(None);
        }
    }

    fn bind_to_tree(&self, tree_in_doc: bool) {
        match self.super_type() {
            Some(ref s) => s.bind_to_tree(tree_in_doc),
            _ => (),
        }

        // When an iframe element is inserted into a document that has a
        // browsing context, the user agent must create a nested browsing
        // context, and then process the iframe attributes for the
        // "first time".
        if tree_in_doc /* && self.owner_doc().has_browser_context() */ {
            *self.browser_context.borrow_mut() = Some(BrowserContext::new());
            self.process_the_iframe_attributes(true);
        }
    }
}

impl Reflectable for HTMLIFrameElement {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.htmlelement.reflector()
    }
}
