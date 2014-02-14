/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use servo_util::namespace;
use dom::attr::Attr;
use dom::node::NodeIterator;
use dom::node::{DoctypeNodeTypeId, DocumentFragmentNodeTypeId, CommentNodeTypeId};
use dom::node::{DocumentNodeTypeId, ElementNodeTypeId, ProcessingInstructionNodeTypeId};
use dom::node::{TextNodeTypeId, AbstractNode};
use servo_util::str::DOMString;

pub fn serialize(iterator: &mut NodeIterator) -> DOMString {
    let mut html = DOMString::empty();
    let mut open_elements: ~[DOMString] = ~[];

    for node in *iterator {
        while open_elements.len() > iterator.depth {
            html.push_str(DOMString::from_strings([
                DOMString::from_string("</"),
                open_elements.pop(),
                DOMString::from_string(">"),
            ]));
        }
        html.push_str(
            match node.type_id() {
                ElementNodeTypeId(..) => {
                    serialize_elem(node, &mut open_elements)
                }
                CommentNodeTypeId => {
                    serialize_comment(node)
                }
                TextNodeTypeId => {
                    serialize_text(node)
                }
                DoctypeNodeTypeId => {
                    serialize_doctype(node)
                }
                ProcessingInstructionNodeTypeId => {
                    serialize_processing_instruction(node)
                }
                DocumentFragmentNodeTypeId => {
                    DOMString::empty()
                }
                DocumentNodeTypeId(_) => {
                    fail!("It shouldn't be possible to serialize a document node")
                }
            }
            );
    }
    while open_elements.len() > 0 {
        html.push_str(DOMString::from_strings([
            DOMString::from_string("</"),
            open_elements.pop(),
            DOMString::from_string(">"),
        ]));
    }
    html
}

fn serialize_comment(node: AbstractNode) -> DOMString {
    node.with_imm_characterdata(|comment| {
        DOMString::from_strings([
            DOMString::from_string("<!--"),
            comment.data.clone(),
            DOMString::from_string("-->"),
        ])
    })
}

fn serialize_text(node: AbstractNode) -> DOMString {
    node.with_imm_characterdata(|text| {
        match node.parent_node() {
            Some(parent) if parent.is_element() => {
                parent.with_imm_element(|elem| {
                    match elem.tag_name.to_string().as_slice() {
                        "style" | "script" | "xmp" | "iframe" |
                        "noembed" | "noframes" | "plaintext" |
                        "noscript" if elem.namespace == namespace::HTML => {
                            text.data.clone()
                        },
                        _ => escape(text.data.clone(), false)
                    }
               })
            },
            _ => escape(text.data.clone(), false)
        }
    })
}

fn serialize_processing_instruction(node: AbstractNode) -> DOMString {
    node.with_imm_processing_instruction(|processing_instruction| {
        DOMString::from_strings([
            DOMString::from_string("<?"),
            processing_instruction.target.clone(),
            DOMString::from_string(" "),
            processing_instruction.characterdata.data.clone(),
            DOMString::from_string("?>"),
        ])
    })
}

fn serialize_doctype(node: AbstractNode) -> DOMString {
    node.with_imm_doctype(|doctype| {
        DOMString::from_strings([
            DOMString::from_string("<!DOCTYPE"), // XXX space
            doctype.name.clone(),
            DOMString::from_string(">"),
        ])
    })
}

fn serialize_elem(node: AbstractNode, open_elements: &mut ~[DOMString]) -> DOMString {
    node.with_imm_element(|elem| {
        let mut rv = DOMString::from_strings([
            DOMString::from_string("<"),
            elem.tag_name.clone()
        ]);
        for attr in elem.attrs.iter() {
            rv.push_str(serialize_attr(attr));
        };
        rv.push_str(DOMString::from_string(">"));
        match elem.tag_name.to_string().as_slice() {
            "pre" | "listing" | "textarea" if
                elem.namespace == namespace::HTML => {
                    match node.first_child() {
                        Some(child) if child.is_text() => {
                            child.with_imm_characterdata(|text| {
                                if text.data[0] == 0x0A as u16 {
                                    rv.push_str(DOMString::from_string("\x0A"));
                                }
                            })
                        },
                        _ => {}
                    }
            },
            _ => {}
        }
        if !elem.is_void() {
            open_elements.push(elem.tag_name.clone());
        }
        rv
    })
}

fn serialize_attr(attr: &@mut Attr) -> DOMString {
    let attr_name = if attr.namespace == namespace::XML {
        DOMString::from_strings([
            DOMString::from_string("xml:"),
            attr.local_name.clone(),
        ])
    } else if attr.namespace == namespace::XMLNS &&
        attr.local_name == DOMString::from_string("xmlns") {
          DOMString::from_string("xmlns")
    } else if attr.namespace == namespace::XMLNS {
        DOMString::from_strings([
            DOMString::from_string("xmlns:"),
            attr.local_name.clone(),
        ])
    } else if attr.namespace == namespace::XLink {
        DOMString::from_strings([
            DOMString::from_string("xlink:"),
            attr.local_name.clone(),
        ])
    } else {
        attr.name.clone()
    };
    DOMString::from_strings([
        DOMString::from_string(" "),
        attr_name,
        DOMString::from_string("=\""),
        escape(attr.value.clone(), true),
        DOMString::from_string("\""),
    ])
}

fn escape(_string: DOMString, _attr_mode: bool) -> DOMString {
    /*let replaced = string.replace("&", "&amp;").replace("\xA0", "&nbsp;");
    match attr_mode {
        true => {
            replaced.replace("\"", "&quot;")
        },
        false => {
            replaced.replace("<", "&lt;").replace(">", "&gt;")
        }
    }*/
    fail!()
}
