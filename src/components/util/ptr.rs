/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub struct MutNonNull<T> {
    ptr: *mut T,
}

impl<T> MutNonNull<T> {
    pub fn new(ptr: *mut T) -> MutNonNull<T> {
        assert!(ptr.is_not_null());
        MutNonNull {
            ptr: ptr,
        }
    }

    pub fn is_null() {}
    pub fn is_not_null() {}
}

impl<T> Deref<*mut T> for MutNonNull<T> {
    fn deref<'a>(&'a self) -> &'a *mut T {
        &self.ptr
    }
}
