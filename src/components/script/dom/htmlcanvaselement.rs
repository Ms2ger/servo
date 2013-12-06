/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::HTMLCanvasElementBinding;
use dom::bindings::codegen::InheritTypes::{HTMLCanvasElementDerived, HTMLElementCast};
use dom::bindings::codegen::InheritTypes::ElementCast;
use dom::bindings::js::{JS, JSRef, Temporary};
use dom::bindings::trace::Traceable;
use dom::bindings::utils::{Reflectable, Reflector};
use dom::canvasrenderingcontext2d::CanvasRenderingContext2D;
use dom::document::Document;
use dom::element::{Element, HTMLCanvasElementTypeId, AttributeHandlers};
use dom::eventtarget::{EventTarget, NodeTargetTypeId};
use dom::htmlelement::HTMLElement;
use dom::node::{Node, ElementNodeTypeId, window_from_node};
use dom::virtualmethods::VirtualMethods;
use geom::size::Size2D;
use servo_util::str::DOMString;

use std::cell::Cell;
use std::num;

static DefaultWidth: u32 = 300;
static DefaultHeight: u32 = 150;

#[deriving(Encodable)]
pub struct HTMLCanvasElement {
    pub htmlelement: HTMLElement,
    context: Traceable<Cell<Option<JS<CanvasRenderingContext2D>>>>,
    width: Traceable<Cell<u32>>,
    height: Traceable<Cell<u32>>,
}

impl HTMLCanvasElementDerived for EventTarget {
    fn is_htmlcanvaselement(&self) -> bool {
        self.type_id == NodeTargetTypeId(ElementNodeTypeId(HTMLCanvasElementTypeId))
    }
}

impl HTMLCanvasElement {
    pub fn new_inherited(localName: DOMString, document: &JSRef<Document>) -> HTMLCanvasElement {
        HTMLCanvasElement {
            htmlelement: HTMLElement::new_inherited(HTMLCanvasElementTypeId, localName, document),
            context: None,
            width: Traceable::new(Cell::new(DefaultWidth)),
            height: Traceable::new(Cell::new(DefaultHeight)),
       }
    }

    pub fn new(localName: DOMString, document: &JSRef<Document>) -> Temporary<HTMLCanvasElement> {
        let element = HTMLCanvasElement::new_inherited(localName, document);
        Node::reflect_node(box element, document, HTMLCanvasElementBinding::Wrap)
    }
}

pub trait HTMLCanvasElementMethods {
    fn Width(&self) -> u32;
    fn SetWidth(&self, width: u32);
    fn Height(&self) -> u32;
    fn SetHeight(&self, height: u32);
    fn GetContext(&self, id: DOMString) -> Option<Temporary<CanvasRenderingContext2D>>;
}

impl<'a> HTMLCanvasElementMethods for JSRef<'a, HTMLCanvasElement> {
    fn Width(&self) -> u32 {
        self.width
    }

    fn SetWidth(&self, width: u32) {
        let elem: &JSRef<Element> = ElementCast::from(self);
        elem.set_uint_attribute("width", width)
    }

    fn Height(&self) -> u32 {
        self.height
    }

    pub fn SetHeight(&self, height: u32) {
        let elem: &JSRef<Element> = ElementCast::from(self);
        elem.set_uint_attribute("height", height)
    }

    pub fn GetContext(&self, id: DOMString) -> Option<Temporary<CanvasRenderingContext2D>> {
        if "2d" != id {
            return None;
        }

        if self.context.get().is_none() {
            let window = window_from_node(self).root();
            let (w, h) = (self.width.get() as i32, self.height.get() as i32);
            self.context.assign(Some(CanvasRenderingContext2D::new(&*window, Size2D(w, h))));
        }
        self.context.get()
     }
}

impl<'a> VirtualMethods for JSRef<'a, HTMLCanvasElement> {
    fn super_type<'a>(&'a self) -> Option<&'a VirtualMethods+> {
        let element: &JSRef<HTMLElement> = HTMLElementCast::from_ref(self);
        Some(element as &VirtualMethods+)
    }

    fn before_remove_attr(&self, name: DOMString, value: DOMString) {
        match self.super_type() {
            Some(ref s) => s.before_remove_attr(name.clone(), value.clone()),
            _ => (),
        }

        let recreate = match name.as_slice() {
            "width" => {
                self.get_mut().width = DefaultWidth;
                true
            }
            "height" => {
                self.get_mut().height = DefaultHeight;
                true
            }
            _ => false,
        };

        if recreate {
            let (w, h) = (self.width.get() as i32, self.height.get() as i32);
            match self.context {
                Some(ref context) => context.recreate(Size2D(w, h)),
                None => ()
            }
        }
    }

    fn after_set_attr(&self, name: DOMString, value: DOMString) {
        match self.super_type() {
            Some(ref s) => s.after_set_attr(name.clone(), value.clone()),
            _ => (),
        }

        let recreate = match name.as_slice() {
            "width" => {
                self.width.set(num::from_str_radix(value, 10).unwrap());
                true
            }
            "height" => {
                self.height.set(num::from_str_radix(value, 10).unwrap());
                true
            }
            _ => false,
        };

        if recreate {
            let (w, h) = (self.width.get() as i32, self.height.get() as i32);
            match self.context {
                Some(ref context) => context.recreate(Size2D(w, h)),
                None => ()
            }
        }
    }
}

impl Reflectable for HTMLCanvasElement {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.htmlelement.reflector()
    }
}
