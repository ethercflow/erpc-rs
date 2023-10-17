// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::pin::Pin;

use erpc_sys::erpc::{self, Timely as RawTimely};

pub struct Timely {
    inner: *mut RawTimely,
}

impl Timely {
    #[inline]
    pub fn from_inner_raw(raw: *mut erpc::Timely) -> Self {
        Timely { inner: raw }
    }

    #[inline]
    pub fn set_rate(&mut self, rate: f64) {
        self.as_inner_mut().set_rate(rate);
    }

    #[inline]
    pub fn get_rtt_perc(&mut self, perc: f64) -> f64 {
        self.as_inner_mut().get_rtt_perc(perc)
    }

    #[inline]
    pub fn reset_rtt_stats(&mut self) {
        self.as_inner_mut().reset_rtt_stats();
    }

    #[inline]
    pub fn get_rate_gbps(&self) -> f64 {
        self.as_inner().get_rate_gbps()
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut RawTimely> {
        unsafe { Pin::new_unchecked(&mut *self.inner) }
    }

    #[inline]
    pub fn as_inner(&self) -> &RawTimely {
        unsafe { &*self.inner }
    }
}
