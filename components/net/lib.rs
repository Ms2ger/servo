/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![feature(box_syntax)]
#![feature(custom_attribute)]
#![feature(custom_derive)]
#![feature(fnbox)]
#![feature(mpsc_select)]
#![feature(plugin)]
#![plugin(plugins)]

#![deny(unsafe_code)]

#[macro_use]
extern crate bitflags;
extern crate brotli;
extern crate content_blocker as content_blocker_parser;
extern crate cookie as cookie_rs;
extern crate device;
extern crate devtools_traits;
extern crate flate2;
extern crate hyper;
extern crate hyper_serde;
extern crate immeta;
extern crate ipc_channel;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] #[no_link] extern crate matches;
#[macro_use]
extern crate mime;
extern crate mime_guess;
extern crate msg;
extern crate net_traits;
extern crate openssl;
extern crate openssl_verify;
extern crate profile_traits;
extern crate rand;
extern crate rustc_serialize;
extern crate threadpool;
extern crate time;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
extern crate tinyfiledialogs;
extern crate unicase;
extern crate url;
extern crate util;
extern crate uuid;
extern crate webrender_traits;
extern crate websocket;

mod about_loader;
mod blob_loader;
mod bluetooth_thread;
mod chrome_loader;
mod connector;
mod content_blocker;
mod cookie;
mod cookie_storage;
mod data_loader;
mod file_loader;
mod filemanager_thread;
mod hsts;
mod http_loader;
mod image_cache_thread;
mod mime_classifier;
mod resource_thread;
mod storage_thread;
mod websocket_loader;

/// An implementation of the [Fetch specification](https://fetch.spec.whatwg.org/)
mod fetch {
    pub mod cors_cache;
    pub mod methods;
}

pub use bluetooth_thread::BluetoothThreadFactory;
pub use image_cache_thread::new_image_cache_thread;
pub use resource_thread::new_resource_threads;
