/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::error::Fallible;
use dom::bindings::js::{JSRef, Temporary};
use dom::bindings::utils::{Reflectable, Reflector};
use dom::eventtarget::EventTarget;
use dom::window::Window;

use servo_util::str::DOMString;

#[deriving(Encodable)]
pub struct Worker {
    eventtarget: EventTarget,
}

impl Worker {
    pub fn Constructor(window: &JSRef<Window>, scriptURL: DOMString) -> Fallible<Temporary<Worker>> {
        fail!()
    }
}

pub trait WorkerMethods {
}

impl Reflectable for Worker {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        self.eventtarget.reflector()
    }
}
