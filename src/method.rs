// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::codec::{DeserializeFn, Marshaller, SerializeFn};

pub struct Method<Req, Resp> {
    /// The unique id of the method.
    pub id: u8,

    /// The marshaller used for request messages.
    pub req_mar: Marshaller<Req>,

    /// The marshaller used for response messages.
    pub resp_mar: Marshaller<Resp>,
}

impl<Req, Resp> Method<Req, Resp> {
    /// Get the request serializer.
    #[inline]
    pub fn req_ser(&self) -> SerializeFn<Req> {
        self.req_mar.ser
    }

    /// Get the request deserializer.
    #[inline]
    pub fn req_de(&self) -> DeserializeFn<Req> {
        self.req_mar.de
    }

    /// Get the response serializer.
    #[inline]
    pub fn resp_ser(&self) -> SerializeFn<Resp> {
        self.resp_mar.ser
    }

    /// Get the response deserializer.
    #[inline]
    pub fn resp_de(&self) -> DeserializeFn<Resp> {
        self.resp_mar.de
    }
}
