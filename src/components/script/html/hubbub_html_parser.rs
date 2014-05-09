/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::AttrBinding::AttrMethods;
use dom::bindings::codegen::Bindings::NodeBinding::NodeMethods;
use dom::bindings::codegen::InheritTypes::{NodeBase, NodeCast, TextCast, ElementCast};
use dom::bindings::js::{JS, JSRef, Temporary, OptionalRootable, Root};
use dom::bindings::utils::Reflectable;
use dom::document::{Document, DocumentHelpers};
use dom::element::{AttributeHandlers, HTMLLinkElementTypeId};
use dom::htmlelement::HTMLElement;
use dom::htmlheadingelement::{Heading1, Heading2, Heading3, Heading4, Heading5, Heading6};
use dom::htmlformelement::HTMLFormElement;
use dom::node::{ElementNodeTypeId, NodeHelpers};
use dom::types::*;
use html::cssparse::{StylesheetProvenance, UrlProvenance, spawn_css_parser};
use page::Page;

use hubbub::hubbub;
use hubbub::hubbub::{NullNs, HtmlNs, MathMlNs, SvgNs, XLinkNs, XmlNs, XmlNsNs};
use servo_net::resource_task::{Load, LoadData, Payload, Done, ResourceTask, load_whole_resource};
use servo_util::namespace;
use servo_util::namespace::{Namespace, Null};
use servo_util::str::{DOMString, HTML_SPACE_CHARACTERS};
use servo_util::task::spawn_named;
use std::ascii::StrAsciiExt;
use std::mem;
use std::cell::RefCell;
use std::comm::{channel, Sender, Receiver};
use style::Stylesheet;
use url::{Url, UrlParser};

macro_rules! handle_element(
    ($document: expr,
     $localName: expr,
     $string: expr,
     $ctor: ident
     $(, $arg:expr )*) => (
        if $string == $localName.as_slice() {
            return ElementCast::from_temporary($ctor::new($localName, $document $(, $arg)*));
        }
    )
)


pub struct JSFile {
    pub data: String,
    pub url: Url
}

pub type JSResult = Vec<JSFile>;

enum CSSMessage {
    CSSTaskNewFile(StylesheetProvenance),
    CSSTaskExit
}

enum JSMessage {
    JSTaskNewFile(Url),
    JSTaskNewInlineScript(String, Url),
    JSTaskExit
}

/// Messages generated by the HTML parser upon discovery of additional resources
pub enum HtmlDiscoveryMessage {
    HtmlDiscoveredStyle(Stylesheet),
    HtmlDiscoveredScript(JSResult)
}

pub struct HtmlParserResult {
    pub discovery_port: Receiver<HtmlDiscoveryMessage>,
}

trait NodeWrapping<T> {
    unsafe fn to_hubbub_node(&self) -> hubbub::NodeDataPtr;
}

impl<'a, T: NodeBase+Reflectable> NodeWrapping<T> for JSRef<'a, T> {
    unsafe fn to_hubbub_node(&self) -> hubbub::NodeDataPtr {
        mem::transmute(self.deref())
    }
}

unsafe fn from_hubbub_node<T: Reflectable>(n: hubbub::NodeDataPtr) -> Temporary<T> {
    Temporary::new(JS::from_raw(mem::transmute(n)))
}

/**
Runs a task that coordinates parsing links to css stylesheets.

This function should be spawned in a separate task and spins waiting
for the html builder to find links to css stylesheets and sends off
tasks to parse each link.  When the html process finishes, it notifies
the listener, who then collects the css rules from each task it
spawned, collates them, and sends them to the given result channel.

# Arguments

* `to_parent` - A channel on which to send back the full set of rules.
* `from_parent` - A port on which to receive new links.

*/
fn css_link_listener(to_parent: Sender<HtmlDiscoveryMessage>,
                     from_parent: Receiver<CSSMessage>) {
    let mut result_vec = vec!();

    loop {
        match from_parent.recv_opt() {
            Ok(CSSTaskNewFile(provenance)) => {
                result_vec.push(spawn_css_parser(provenance));
            }
            Ok(CSSTaskExit) | Err(()) => {
                break;
            }
        }
    }

    // Send the sheets back in order
    // FIXME: Shouldn't wait until after we've recieved CSSTaskExit to start sending these
    for port in result_vec.iter() {
        assert!(to_parent.send_opt(HtmlDiscoveredStyle(port.recv())).is_ok());
    }
}

fn js_script_listener(to_parent: Sender<HtmlDiscoveryMessage>,
                      from_parent: Receiver<JSMessage>,
                      resource_task: ResourceTask) {
    let mut result_vec = vec!();

    loop {
        match from_parent.recv_opt() {
            Ok(JSTaskNewFile(url)) => {
                match load_whole_resource(&resource_task, url.clone()) {
                    Err(_) => {
                        error!("error loading script {:s}", url.serialize());
                    }
                    Ok((metadata, bytes)) => {
                        result_vec.push(JSFile {
                            data: String::from_utf8(bytes).unwrap().to_string(),
                            url: metadata.final_url,
                        });
                    }
                }
            }
            Ok(JSTaskNewInlineScript(data, url)) => {
                result_vec.push(JSFile { data: data, url: url });
            }
            Ok(JSTaskExit) | Err(()) => {
                break;
            }
        }
    }

    assert!(to_parent.send_opt(HtmlDiscoveredScript(result_vec)).is_ok());
}

// Silly macros to handle constructing      DOM nodes. This produces bad code and should be optimized
// via atomization (issue #85).

pub fn build_element_from_tag(tag: DOMString, ns: Namespace, document: &JSRef<Document>) -> Temporary<Element> {
    if ns != namespace::HTML {
        return Element::new(tag, ns, None, document);
    }

    // TODO (Issue #85): use atoms
    handle_element!(document, tag, "a",         HTMLAnchorElement);
    handle_element!(document, tag, "abbr",      HTMLElement);
    handle_element!(document, tag, "acronym",   HTMLElement);
    handle_element!(document, tag, "address",   HTMLElement);
    handle_element!(document, tag, "applet",    HTMLAppletElement);
    handle_element!(document, tag, "area",      HTMLAreaElement);
    handle_element!(document, tag, "article",   HTMLElement);
    handle_element!(document, tag, "aside",     HTMLElement);
    handle_element!(document, tag, "audio",     HTMLAudioElement);
    handle_element!(document, tag, "b",         HTMLElement);
    handle_element!(document, tag, "base",      HTMLBaseElement);
    handle_element!(document, tag, "bdi",       HTMLElement);
    handle_element!(document, tag, "bdo",       HTMLElement);
    handle_element!(document, tag, "bgsound",   HTMLElement);
    handle_element!(document, tag, "big",       HTMLElement);
    handle_element!(document, tag, "blockquote",HTMLElement);
    handle_element!(document, tag, "body",      HTMLBodyElement);
    handle_element!(document, tag, "br",        HTMLBRElement);
    handle_element!(document, tag, "button",    HTMLButtonElement);
    handle_element!(document, tag, "canvas",    HTMLCanvasElement);
    handle_element!(document, tag, "caption",   HTMLTableCaptionElement);
    handle_element!(document, tag, "center",    HTMLElement);
    handle_element!(document, tag, "cite",      HTMLElement);
    handle_element!(document, tag, "code",      HTMLElement);
    handle_element!(document, tag, "col",       HTMLTableColElement);
    handle_element!(document, tag, "colgroup",  HTMLTableColElement);
    handle_element!(document, tag, "data",      HTMLDataElement);
    handle_element!(document, tag, "datalist",  HTMLDataListElement);
    handle_element!(document, tag, "dd",        HTMLElement);
    handle_element!(document, tag, "del",       HTMLModElement);
    handle_element!(document, tag, "details",   HTMLElement);
    handle_element!(document, tag, "dfn",       HTMLElement);
    handle_element!(document, tag, "dir",       HTMLDirectoryElement);
    handle_element!(document, tag, "div",       HTMLDivElement);
    handle_element!(document, tag, "dl",        HTMLDListElement);
    handle_element!(document, tag, "dt",        HTMLElement);
    handle_element!(document, tag, "em",        HTMLElement);
    handle_element!(document, tag, "embed",     HTMLEmbedElement);
    handle_element!(document, tag, "fieldset",  HTMLFieldSetElement);
    handle_element!(document, tag, "figcaption",HTMLElement);
    handle_element!(document, tag, "figure",    HTMLElement);
    handle_element!(document, tag, "font",      HTMLFontElement);
    handle_element!(document, tag, "footer",    HTMLElement);
    handle_element!(document, tag, "form",      HTMLFormElement);
    handle_element!(document, tag, "frame",     HTMLFrameElement);
    handle_element!(document, tag, "frameset",  HTMLFrameSetElement);
    handle_element!(document, tag, "h1",        HTMLHeadingElement, Heading1);
    handle_element!(document, tag, "h2",        HTMLHeadingElement, Heading2);
    handle_element!(document, tag, "h3",        HTMLHeadingElement, Heading3);
    handle_element!(document, tag, "h4",        HTMLHeadingElement, Heading4);
    handle_element!(document, tag, "h5",        HTMLHeadingElement, Heading5);
    handle_element!(document, tag, "h6",        HTMLHeadingElement, Heading6);
    handle_element!(document, tag, "head",      HTMLHeadElement);
    handle_element!(document, tag, "header",    HTMLElement);
    handle_element!(document, tag, "hgroup",    HTMLElement);
    handle_element!(document, tag, "hr",        HTMLHRElement);
    handle_element!(document, tag, "html",      HTMLHtmlElement);
    handle_element!(document, tag, "i",         HTMLElement);
    handle_element!(document, tag, "iframe",    HTMLIFrameElement);
    handle_element!(document, tag, "img",       HTMLImageElement);
    handle_element!(document, tag, "input",     HTMLInputElement);
    handle_element!(document, tag, "ins",       HTMLModElement);
    handle_element!(document, tag, "isindex",   HTMLElement);
    handle_element!(document, tag, "kbd",       HTMLElement);
    handle_element!(document, tag, "label",     HTMLLabelElement);
    handle_element!(document, tag, "legend",    HTMLLegendElement);
    handle_element!(document, tag, "li",        HTMLLIElement);
    handle_element!(document, tag, "link",      HTMLLinkElement);
    handle_element!(document, tag, "main",      HTMLElement);
    handle_element!(document, tag, "map",       HTMLMapElement);
    handle_element!(document, tag, "mark",      HTMLElement);
    handle_element!(document, tag, "marquee",   HTMLElement);
    handle_element!(document, tag, "meta",      HTMLMetaElement);
    handle_element!(document, tag, "meter",     HTMLMeterElement);
    handle_element!(document, tag, "nav",       HTMLElement);
    handle_element!(document, tag, "nobr",      HTMLElement);
    handle_element!(document, tag, "noframes",  HTMLElement);
    handle_element!(document, tag, "noscript",  HTMLElement);
    handle_element!(document, tag, "object",    HTMLObjectElement);
    handle_element!(document, tag, "ol",        HTMLOListElement);
    handle_element!(document, tag, "optgroup",  HTMLOptGroupElement);
    handle_element!(document, tag, "option",    HTMLOptionElement);
    handle_element!(document, tag, "output",    HTMLOutputElement);
    handle_element!(document, tag, "p",         HTMLParagraphElement);
    handle_element!(document, tag, "param",     HTMLParamElement);
    handle_element!(document, tag, "pre",       HTMLPreElement);
    handle_element!(document, tag, "progress",  HTMLProgressElement);
    handle_element!(document, tag, "q",         HTMLQuoteElement);
    handle_element!(document, tag, "rp",        HTMLElement);
    handle_element!(document, tag, "rt",        HTMLElement);
    handle_element!(document, tag, "ruby",      HTMLElement);
    handle_element!(document, tag, "s",         HTMLElement);
    handle_element!(document, tag, "samp",      HTMLElement);
    handle_element!(document, tag, "script",    HTMLScriptElement);
    handle_element!(document, tag, "section",   HTMLElement);
    handle_element!(document, tag, "select",    HTMLSelectElement);
    handle_element!(document, tag, "small",     HTMLElement);
    handle_element!(document, tag, "source",    HTMLSourceElement);
    handle_element!(document, tag, "spacer",    HTMLElement);
    handle_element!(document, tag, "span",      HTMLSpanElement);
    handle_element!(document, tag, "strike",    HTMLElement);
    handle_element!(document, tag, "strong",    HTMLElement);
    handle_element!(document, tag, "style",     HTMLStyleElement);
    handle_element!(document, tag, "sub",       HTMLElement);
    handle_element!(document, tag, "summary",   HTMLElement);
    handle_element!(document, tag, "sup",       HTMLElement);
    handle_element!(document, tag, "table",     HTMLTableElement);
    handle_element!(document, tag, "tbody",     HTMLTableSectionElement);
    handle_element!(document, tag, "td",        HTMLTableDataCellElement);
    handle_element!(document, tag, "template",  HTMLTemplateElement);
    handle_element!(document, tag, "textarea",  HTMLTextAreaElement);
    handle_element!(document, tag, "th",        HTMLTableHeaderCellElement);
    handle_element!(document, tag, "time",      HTMLTimeElement);
    handle_element!(document, tag, "title",     HTMLTitleElement);
    handle_element!(document, tag, "tr",        HTMLTableRowElement);
    handle_element!(document, tag, "tt",        HTMLElement);
    handle_element!(document, tag, "track",     HTMLTrackElement);
    handle_element!(document, tag, "u",         HTMLElement);
    handle_element!(document, tag, "ul",        HTMLUListElement);
    handle_element!(document, tag, "var",       HTMLElement);
    handle_element!(document, tag, "video",     HTMLVideoElement);
    handle_element!(document, tag, "wbr",       HTMLElement);

    return ElementCast::from_temporary(HTMLUnknownElement::new(tag, document));
}

pub fn parse_html(page: &Page,
                  document: &JSRef<Document>,
                  url: Url,
                  resource_task: ResourceTask)
                  -> HtmlParserResult {
    debug!("Hubbub: parsing {:?}", url);
    // Spawn a CSS parser to receive links to CSS style sheets.

    let (discovery_chan, discovery_port) = channel();
    let stylesheet_chan = discovery_chan.clone();
    let (css_chan, css_msg_port) = channel();
    spawn_named("parse_html:css", proc() {
        css_link_listener(stylesheet_chan, css_msg_port);
    });

    // Spawn a JS parser to receive JavaScript.
    let resource_task2 = resource_task.clone();
    let js_result_chan = discovery_chan.clone();
    let (js_chan, js_msg_port) = channel();
    spawn_named("parse_html:js", proc() {
        js_script_listener(js_result_chan, js_msg_port, resource_task2.clone());
    });

    // Wait for the LoadResponse so that the parser knows the final URL.
    let (input_chan, input_port) = channel();
    resource_task.send(Load(LoadData::new(url.clone()), input_chan));
    let load_response = input_port.recv();

    debug!("Fetched page; metadata is {:?}", load_response.metadata);

    let base_url = &load_response.metadata.final_url;

    {
        // Store the final URL before we start parsing, so that DOM routines
        // (e.g. HTMLImageElement::update_image) can resolve relative URLs
        // correctly.
        *page.mut_url() = Some((base_url.clone(), true));
    }

    let mut parser = build_parser(unsafe { document.to_hubbub_node() });
    debug!("created parser");

    let (css_chan2, js_chan2) = (css_chan.clone(), js_chan.clone());

    let doc_cell = RefCell::new(document);

    let mut tree_handler = hubbub::TreeHandler {
        create_comment: |data: String| {
            debug!("create comment");
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow();
            let tmp = &*tmp_borrow;
            let comment = Comment::new(data, *tmp).root();
            let comment: &JSRef<Node> = NodeCast::from_ref(&*comment);
            unsafe { comment.to_hubbub_node() }
        },
        create_doctype: |doctype: Box<hubbub::Doctype>| {
            debug!("create doctype");
            let box hubbub::Doctype {
                name: name,
                public_id: public_id,
                system_id: system_id,
                force_quirks: _
            } = doctype;
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow();
            let tmp = &*tmp_borrow;
            let doctype_node = DocumentType::new(name, public_id, system_id, *tmp).root();
            unsafe {
                doctype_node.deref().to_hubbub_node()
            }
        },
        create_element: |tag: Box<hubbub::Tag>| {
            debug!("create element {}", tag.name);
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow();
            let tmp = &*tmp_borrow;
            let namespace = match tag.ns {
                HtmlNs => namespace::HTML,
                MathMlNs => namespace::MathML,
                SvgNs => namespace::SVG,
                ns => fail!("Not expecting namespace {:?}", ns),
            };
            let element: Root<Element> = build_element_from_tag(tag.name.clone(), namespace, *tmp).root();

            debug!("-- attach attrs");
            for attr in tag.attributes.iter() {
                let (namespace, prefix) = match attr.ns {
                    NullNs => (namespace::Null, None),
                    XLinkNs => (namespace::XLink, Some("xlink")),
                    XmlNs => (namespace::XML, Some("xml")),
                    XmlNsNs => (namespace::XMLNS, Some("xmlns")),
                    ns => fail!("Not expecting namespace {:?}", ns),
                };
                element.set_attribute_from_parser(attr.name.clone(),
                                                  attr.value.clone(),
                                                  namespace,
                                                  prefix.map(|p| p.to_string()));
            }

            //FIXME: workaround for https://github.com/mozilla/rust/issues/13246;
            //       we get unrooting order failures if these are inside the match.
            let rel = {
                let rel = element.deref().get_attribute(Null, "rel").root();
                rel.map(|a| a.deref().Value())
            };
            let href = {
                let href= element.deref().get_attribute(Null, "href").root();
                href.map(|a| a.deref().Value())
            };

            // Spawn additional parsing, network loads, etc. from tag and attrs
            let type_id = {
                let node: &JSRef<Node> = NodeCast::from_ref(&*element);
                node.type_id()
            };
            match type_id {
                // Handle CSS style sheets from <link> elements
                ElementNodeTypeId(HTMLLinkElementTypeId) => {
                    match (rel, href) {
                        (Some(ref rel), Some(ref href)) if rel.as_slice().split(HTML_SPACE_CHARACTERS.as_slice())
                                                              .any(|s| {
                                    s.as_slice().eq_ignore_ascii_case("stylesheet")
                                }) => {
                            debug!("found CSS stylesheet: {:s}", *href);
                            match UrlParser::new().base_url(base_url).parse(href.as_slice()) {
                                Ok(url) => css_chan2.send(CSSTaskNewFile(
                                    UrlProvenance(url, resource_task.clone()))),
                                Err(e) => debug!("Parsing url {:s} failed: {:s}", *href, e)
                            };
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            unsafe { element.deref().to_hubbub_node() }
        },
        create_text: |data: String| {
            debug!("create text");
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow();
            let tmp = &*tmp_borrow;
            let text = Text::new(data, *tmp).root();
            unsafe { text.deref().to_hubbub_node() }
        },
        ref_node: |_| {},
        unref_node: |_| {},
        append_child: |parent: hubbub::NodeDataPtr, child: hubbub::NodeDataPtr| {
            unsafe {
                debug!("append child {:x} {:x}", parent, child);
                let child: Root<Node> = from_hubbub_node(child).root();
                child.init();
                let parent: Root<Node> = from_hubbub_node(parent).root();
                parent.init();
                assert!(parent.deref().AppendChild(&*child).is_ok());
            }
            child
        },
        insert_before: |_parent, _child| {
            debug!("insert before");
            0u
        },
        remove_child: |_parent, _child| {
            debug!("remove child");
            0u
        },
        clone_node: |_node, deep| {
            debug!("clone node");
            if deep { error!("-- deep clone unimplemented"); }
            fail!("clone node unimplemented")
        },
        reparent_children: |_node, _new_parent| {
            debug!("reparent children");
            0u
        },
        get_parent: |_node, _element_only| {
            debug!("get parent");
            0u
        },
        has_children: |_node| {
            debug!("has children");
            false
        },
        form_associate: |_form, _node| {
            debug!("form associate");
        },
        add_attributes: |_node, _attributes| {
            debug!("add attributes");
        },
        set_quirks_mode: |mode| {
            debug!("set quirks mode");
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow_mut();
            let tmp = &*tmp_borrow;
            tmp.set_quirks_mode(mode);
        },
        encoding_change: |encname| {
            debug!("encoding change");
            // NOTE: tmp vars are workaround for lifetime issues. Both required.
            let tmp_borrow = doc_cell.borrow_mut();
            let tmp = &*tmp_borrow;
            tmp.set_encoding_name(encname);
        },
        complete_script: |script| {
            unsafe {
                let script: &JSRef<Element> = &*from_hubbub_node(script).root();
                match script.get_attribute(Null, "src").root() {
                    Some(src) => {
                        debug!("found script: {:s}", src.deref().Value());
                        match UrlParser::new().base_url(base_url)
                                .parse(src.deref().value().as_slice()) {
                            Ok(new_url) => js_chan2.send(JSTaskNewFile(new_url)),
                            Err(e) => debug!("Parsing url {:s} failed: {:s}", src.deref().Value(), e)
                        };
                    }
                    None => {
                        let mut data = String::new();
                        let scriptnode: &JSRef<Node> = NodeCast::from_ref(script);
                        debug!("iterating over children {:?}", scriptnode.first_child());
                        for child in scriptnode.children() {
                            debug!("child = {:?}", child);
                            let text: &JSRef<Text> = TextCast::to_ref(&child).unwrap();
                            data.push_str(text.deref().characterdata.data.deref().borrow().as_slice());
                        }

                        debug!("script data = {:?}", data);
                        js_chan2.send(JSTaskNewInlineScript(data, base_url.clone()));
                    }
                }
            }
            debug!("complete script");
        },
        complete_style: |_| {
            // style parsing is handled in element::notify_child_list_changed.
        },
    };
    parser.set_tree_handler(&mut tree_handler);
    debug!("set tree handler");

    debug!("loaded page");
    loop {
        match load_response.progress_port.recv() {
            Payload(data) => {
                debug!("received data");
                parser.parse_chunk(data.as_slice());
            }
            Done(Err(err)) => {
                fail!("Failed to load page URL {:s}, error: {:s}", url.serialize(), err);
            }
            Done(..) => {
                break;
            }
        }
    }

    debug!("finished parsing");
    css_chan.send(CSSTaskExit);
    js_chan.send(JSTaskExit);

    HtmlParserResult {
        discovery_port: discovery_port,
    }
}

fn build_parser(node: hubbub::NodeDataPtr) -> hubbub::Parser {
    let mut parser = hubbub::Parser::new("UTF-8", false);
    parser.set_document_node(node);
    parser.enable_scripting(true);
    parser.enable_styling(true);
    parser
}

