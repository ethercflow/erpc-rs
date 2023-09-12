// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use erpc_sys::{erpc::{TimingWheel as RawTimingWheel, wheel_record_t}, CxxVector};
use std::pin::Pin;

pub struct TimingWheel {
    inner: *mut RawTimingWheel,
}

impl TimingWheel {
    #[inline]
    pub fn from_inner_raw(raw: *mut RawTimingWheel) -> Self {
        TimingWheel { inner: raw }
    }

    #[inline]
    pub fn get_record_vec(&mut self) -> Pin<&mut CxxVector<wheel_record_t>> {
        self.as_inner_mut().get_record_vec()
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.inner.is_null()
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut RawTimingWheel> {
        unsafe { Pin::new_unchecked(&mut *self.inner) }
    }
}
