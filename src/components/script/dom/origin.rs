/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use url::{Url, Host, RelativeScheme};
use url::whatwg_scheme_type_mapper;

use std::sync::atomics::{AtomicUint, SeqCst, INIT_ATOMIC_UINT};

static mut COUNTER: AtomicUint = INIT_ATOMIC_UINT;

pub enum Origin {
    Opaque(uint),
    URL(String, Host, String),
}

impl Origin {
    // http://url.spec.whatwg.org/#concept-url-origin
    pub fn from_url(url: &Url) -> Origin {
        match url.scheme.as_slice() {
            "blob" => fail!("Leave blob URLs out of this"),
            "ftp" | "gopher" | "http" | "https" | "ws" | "wss" => {
                let port = match url.port() {
                    None | Some("") => {
                        match whatwg_scheme_type_mapper(url.scheme.as_slice()) {
                            RelativeScheme(port) => port.to_string(),
                            _ => fail!("Expected RelativeScheme"),
                        }
                    },
                    Some(port) => port.to_string(),
                };
                URL(url.scheme.clone(), url.host().unwrap().clone(), port)
            }
            "file" | _ => Opaque(unsafe { COUNTER.fetch_add(1, SeqCst) })
        }
    }
}
