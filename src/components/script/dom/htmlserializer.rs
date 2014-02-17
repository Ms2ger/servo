/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use servo_util::namespace;
use dom::attr::Attr;
use dom::node::NodeIterator;
use dom::node::{DoctypeNodeTypeId, DocumentFragmentNodeTypeId, CommentNodeTypeId};
use dom::node::{DocumentNodeTypeId, ElementNodeTypeId, ProcessingInstructionNodeTypeId};
use dom::node::{TextNodeTypeId, AbstractNode};
use servo_util::str::{DOMString, DOMSlice};

pub fn serialize(iterator: &mut NodeIterator) -> DOMString {
    let mut html = DOMString::empty();
    let mut open_elements: ~[DOMString] = ~[];

    for node in *iterator {
        while open_elements.len() > iterator.depth {
            let end_tag = pop_end_tag(&mut open_elements);
            html.push_str(end_tag.as_slice());
        }
        let contents =
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
            };
        html.push_str(contents.as_slice());
    }
    while open_elements.len() > 0 {
        let end_tag = pop_end_tag(&mut open_elements);
        html.push_str(end_tag.as_slice());
    }
    html
}

fn pop_end_tag(open_elements: &mut ~[DOMString]) -> DOMString {
    let start = DOMString::from_string("</");
    let middle = open_elements.pop();
    let end = DOMString::from_string(">");
    start + middle.as_slice() + end.as_slice()
}

fn serialize_comment(node: AbstractNode) -> DOMString {
    node.with_imm_characterdata(|comment| {
        let end = DOMString::from_string("-->");
        DOMString::from_string("<!--") +
        comment.data.as_slice() +
        end.as_slice()
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
                        _ => escape(text.data.as_slice(), false)
                    }
               })
            },
            _ => escape(text.data.as_slice(), false)
        }
    })
}

fn serialize_processing_instruction(node: AbstractNode) -> DOMString {
    node.with_imm_processing_instruction(|processing_instruction| {
        let begin = DOMString::from_string("<?");
        let middle = DOMString::from_string(" ");
        let end = DOMString::from_string("?>");
        begin +
        processing_instruction.target.as_slice() +
        middle.as_slice() +
        processing_instruction.characterdata.data.as_slice() +
        end.as_slice()
    })
}

fn serialize_doctype(node: AbstractNode) -> DOMString {
    node.with_imm_doctype(|doctype| {
        let begin = DOMString::from_string("<!DOCTYPE"); // XXX space
        let end = DOMString::from_string(">");
        begin + doctype.name.as_slice() + end.as_slice()
    })
}

fn serialize_elem(node: AbstractNode, open_elements: &mut ~[DOMString]) -> DOMString {
    node.with_imm_element(|elem| {
        let mut rv = DOMString::from_string("<") +
                     elem.tag_name.as_slice();
        for attr in elem.attrs.iter() {
            let attr = serialize_attr(attr);
            rv.push_str(attr.as_slice());
        }
        {
            let gt = DOMString::from_string(">");
            rv.push_str(gt.as_slice());
        }
        match elem.tag_name.to_string().as_slice() {
            "pre" | "listing" | "textarea" if
                elem.namespace == namespace::HTML => {
                    match node.first_child() {
                        Some(child) if child.is_text() => {
                            child.with_imm_characterdata(|text| {
                                if text.data[0] == 0x0A as u16 {
                                    let nl = DOMString::from_string("\x0A");
                                    rv.push_str(nl.as_slice());
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
        let prefix = DOMString::from_string("xml:");
        prefix + attr.local_name.as_slice()
    } else if attr.namespace == namespace::XMLNS &&
        attr.local_name == DOMString::from_string("xmlns") {
        DOMString::from_string("xmlns")
    } else if attr.namespace == namespace::XMLNS {
        let prefix = DOMString::from_string("xmlns:");
        prefix + attr.local_name.as_slice()
    } else if attr.namespace == namespace::XLink {
        let prefix = DOMString::from_string("xlink:");
        prefix + attr.local_name.as_slice()
    } else {
        attr.name.clone()
    };
    let begin = DOMString::from_string(" ");
    let middle = DOMString::from_string("=\"");
    let end = DOMString::from_string("\"");
    let escaped = escape(attr.value.as_slice(), true);
    begin +
    attr_name.as_slice() +
    middle.as_slice() +
    escaped.as_slice() +
    end.as_slice()
}

fn escape(string: DOMSlice, attr_mode: bool) -> DOMString {
    string.replace(if attr_mode {
        |c| {
            match c {
                0x26 => DOMString::from_string("&amp;"),
                0xA0 => DOMString::from_string("&nbsp;"),
                0x22 => DOMString::from_string("&quot;"),
                _    => DOMString::from_buffer(~[c]),
            }
        }
    } else {
        |c| {
            match c {
                0x26 => DOMString::from_string("&amp;"),
                0xA0 => DOMString::from_string("&nbsp;"),
                0x3C => DOMString::from_string("&lt;"),
                0x3E => DOMString::from_string("&gt;"),
                _    => DOMString::from_buffer(~[c]),
            }
        }
    })
}
