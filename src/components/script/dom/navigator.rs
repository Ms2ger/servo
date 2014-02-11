/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::utils::{Reflectable, Reflector, reflect_dom_object};
use dom::bindings::utils::Fallible;
use dom::bindings::codegen::NavigatorBinding;
use dom::window::Window;
use servo_util::str::DOMString;

pub struct Navigator {
    reflector_: Reflector //XXXjdm cycle: window->navigator->window
}

impl Navigator {
    pub fn new_inherited() -> Navigator {
        Navigator {
            reflector_: Reflector::new()
        }
    }

    pub fn new(window: &Window) -> @mut Navigator {
        reflect_dom_object(@mut Navigator::new_inherited(), window, NavigatorBinding::Wrap)
    }

    pub fn DoNotTrack(&self) -> DOMString {
        DOMString::from_string("unspecified")
    }

    pub fn Vendor(&self) -> DOMString {
        DOMString::empty() // Like Gecko
    }

    pub fn VendorSub(&self) -> DOMString {
        DOMString::empty() // Like Gecko
    }

    pub fn Product(&self) -> DOMString {
        DOMString::from_string("Gecko")
    }

    pub fn ProductSub(&self) -> DOMString {
        DOMString::empty()
    }

    pub fn CookieEnabled(&self) -> bool {
        false
    }

    pub fn GetBuildID(&self) -> Fallible<DOMString> {
        Ok(DOMString::empty())
    }

    pub fn JavaEnabled(&self) -> Fallible<bool> {
        Ok(false)
    }

    pub fn TaintEnabled(&self) -> bool {
        false
    }

    pub fn AppName(&self) -> DOMString {
        DOMString::from_string("Netscape") // Like Gecko/Webkit
    }

    pub fn GetAppCodeName(&self) -> Fallible<DOMString> {
        Ok(DOMString::from_string("Mozilla")) // Like Gecko/Webkit
    }

    pub fn GetAppVersion(&self) -> Fallible<DOMString> {
        Ok(DOMString::empty())
    }

    pub fn GetPlatform(&self) -> Fallible<DOMString> {
        Ok(DOMString::empty())
    }

    pub fn GetUserAgent(&self) -> Fallible<DOMString> {
        Ok(DOMString::empty())
    }

    pub fn GetLanguage(&self) -> Option<DOMString> {
        None
    }

    pub fn OnLine(&self) -> bool {
        true
    }
}

impl Reflectable for Navigator {
    fn reflector<'a>(&'a self) -> &'a Reflector {
        &self.reflector_
    }

    fn mut_reflector<'a>(&'a mut self) -> &'a mut Reflector {
        &mut self.reflector_
    }
}
