/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//use std::ascii::Ascii;
use std::str;
use std::to_bytes::{IterBytes, Cb};
//use std::vec::SplitIterator;

//#[deriving(Equiv)]
pub struct DOMChar(u16);
pub struct DOMString(~[u16]);
pub struct DOMSlice<'a>(&'a [u16]);

impl DOMString {
    pub fn empty() -> DOMString {
        DOMString(~[])
    }

    pub fn from_string(s: &str) -> DOMString {
        DOMString(s.to_utf16())
    }

    pub fn push_str(&mut self, s: DOMSlice) {
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
        self.as_slice().to_ascii_lower()
    }

    pub fn to_ascii_upper(&self) -> DOMString {
        self.as_slice().to_ascii_upper()
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

    pub fn to_string(&self) -> ~str {
        str::from_utf16(**self)
    }

    pub fn ascii_lower_char(b: u16) -> u16 {
        if 'A' as u16 <= b && b <= 'Z' as u16 {
            b + ('a' as u16 - 'A' as u16)
        } else {
            b
        }
    }

    pub fn ascii_upper_char(b: u16) -> u16 {
        if 'a' as u16 <= b && b <= 'z' as u16 {
            b - ('a' as u16 - 'A' as u16)
        } else {
            b
        }
    }

    pub fn to_ascii_lower(&self) -> DOMString {
        let bytes = (**self).iter()
                            .map(|&b| DOMSlice::ascii_lower_char(b))
                            .to_owned_vec();
        DOMString(bytes)
    }

    pub fn to_ascii_upper(&self) -> DOMString {
        let bytes = (**self).iter()
                            .map(|&b| DOMSlice::ascii_upper_char(b))
                            .to_owned_vec();
        DOMString(bytes)
    }

    pub fn eq_ignore_ascii_case(&self, other: DOMSlice) -> bool {
        self.len() == other.len() &&
        self.iter().zip(other.iter()).all(|(&s, &o)| {
            s == o ||
            DOMSlice::ascii_lower_char(s) == DOMSlice::ascii_lower_char(o)
        })
    }

    pub fn starts_with(&self, other: DOMSlice) -> bool {
        (**self).starts_with(*other)
    }

    pub fn ends_with(&self, other: DOMSlice) -> bool {
        (**self).ends_with(*other)
    }

/*    pub fn contains(&self, other: DOMSlice) -> bool {
        (**self).contains(*other)
    }*/

    /*pub fn split(&'a self, separators: &'a [Ascii]) -> SplitIterator<'a, u16> {
        (**self).split(|&c| {
            let maybe_ascii = DOMChar(c).to_ascii_opt();
            maybe_ascii.map_default(false, |a| separators.contains(&a))
        })
    }*/

//    pub fn from_string(s: &str) -> DOMString {
//        s.to_utf16()
//    }
}

impl<'a> Eq for DOMSlice<'a> {
    fn eq(&self, other: &DOMSlice<'a>) -> bool {
        (**self).eq(&**other)
    }
}

impl<'a> IterBytes for DOMSlice<'a> {
    fn iter_bytes(&self, lsb0: bool, f: Cb) -> bool {
        (**self).iter_bytes(lsb0, f)
    }
}

impl Equiv<DOMString> for DOMString {
    fn equiv(&self, other: &DOMString) -> bool {
        (**self).equiv(&**other)
    }
}

/*impl<'a> Equiv<DOMSlice<'a>> for DOMString {
    fn equiv(&self, other: &DOMSlice<'a>) -> bool {
        (**self).equiv(&**other)
    }
}*/

impl<'a> Equiv<DOMString> for DOMSlice<'a> {
    fn equiv(&self, other: &DOMString) -> bool {
        (**self).equiv(&**other)
    }
}

impl<'a> Add<DOMSlice<'a>, DOMString> for DOMString {
    fn add(&self, rhs: &DOMSlice<'a>) -> DOMString {
        let mut result = self.clone();
        result.push_str(*rhs);
        result
    }
}

pub fn null_str_as_empty(s: &Option<DOMString>) -> DOMString {
    // We don't use map_default because it would allocate ~"" even for Some.
    match *s {
        Some(ref s) => s.clone(),
        None => DOMString::empty(),
    }
}

pub fn null_str_as_empty_ref<'a>(s: &'a Option<DOMString>) -> DOMSlice<'a> {
    match *s {
        Some(ref s) => s.as_slice(),
        None => DOMSlice(&'a []),
    }
}


/*impl AsciiCast<Ascii> for DOMChar {
    #[inline]
    unsafe fn to_ascii_nocheck(&self) -> Ascii {
        Ascii { chr: **self as u8 }
    }

    #[inline]
    fn is_ascii(&self) -> bool {
        **self & 128 == 0u16
    }
}*/
