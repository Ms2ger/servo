/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//#[deriving(Equiv)]
pub struct DOMString(~[u16]);
pub struct DOMSlice<'a>(&'a [u16]);

impl DOMString {
    pub fn empty() -> DOMString {
        DOMString(~[])
    }

    pub fn from_string(s: &str) -> DOMString {
        DOMString(s.to_utf16())
    }

    pub fn from_strings(strings: &[DOMString]) -> DOMString {
        let mut bytes = ~[];
        for s in strings.iter() {
            bytes.push_all(**s);
        }
        DOMString(bytes)
    }

    pub fn push_str(&self, s: DOMString) {
        self.push_all(*s)
    }

    pub fn as_slice<'a>(&'a self) -> DOMSlice<'a> {
        DOMSlice((**self).as_slice())
    }

    pub fn slice<'a>(&'a self, begin: uint, end: uint) -> DOMSlice<'a> {
        DOMSlice((**self).slice(begin, end))
    }

    pub fn to_string(&self) -> ~str {
        str::from_utf16(**self)
    }

    pub fn to_ascii_lower(&self) -> DOMString {
        let bytes = (**self).iter().map(|&b| {
            if 'A' as u16 <= b && b <= 'Z' as u16 {
                b + ('a' as u16 - 'A' as u16)
            } else {
                b
            }
        }).to_owned_vec();
        DOMString(bytes)
    }

    pub fn to_ascii_upper(&self) -> DOMString {
        let bytes = (**self).iter().map(|&b| {
            if 'a' as u16 <= b && b <= 'z' as u16 {
                b - ('a' as u16 - 'A' as u16)
            } else {
                b
            }
        }).to_owned_vec();
        DOMString(bytes)
    }
}

impl Clone for DOMString {
    fn clone(&self) -> DOMString {
        DOMString((**self).clone())
    }
}

impl Eq for DOMString {
    fn eq(&self, other: &DOMString) -> bool {
        (**self).eq(&**other)
    }
}

impl IterBytes for DOMString {
    fn iter_bytes(&self, lsb0: bool, f: Cb) -> bool {
        (**self).iter_bytes(lsb0, f)
    }
}

impl<'a> DOMSlice<'a> {
    pub fn empty() -> DOMSlice<'a> {
        DOMSlice(&'a [])
    }

    pub fn to_str(&self) -> DOMString {
        DOMString((**self).to_owned())
    }
//    pub fn from_string(s: &str) -> DOMString {
//        s.to_utf16()
//    }
}

impl Equiv<DOMString> for DOMString {
    fn equiv(&self, other: DOMString) -> bool {
        (**self).equiv(&**other)
    }
}

impl<'a> Equiv<DOMSlice<'a>> for DOMString {
    fn equiv(&self, other: &DOMSlice<'a>) -> bool {
        (**self).equiv(&**other)
    }
}

pub fn null_str_as_empty(s: &Option<DOMString>) -> DOMString {
    // We don't use map_default because it would allocate ~"" even for Some.
    match *s {
        Some(ref s) => s.clone(),
        None => ~""
    }
}

pub fn null_str_as_empty_ref<'a>(s: &'a Option<DOMString>) -> &'a str {
    match *s {
        Some(ref s) => s.as_slice(),
        None => &'a ""
    }
}

