// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::buf::MsgBufferReader;
use crate::error::Result;
use crate::msg_buffer::MsgBuffer;

pub type DeserializeFn<T> = fn(MsgBufferReader) -> Result<T>;
pub type SerializeFn<T> = fn(&T, &mut MsgBuffer) -> Result<()>;

pub struct Marshaller<T> {
    pub ser: SerializeFn<T>,
    pub de: DeserializeFn<T>,
}

pub mod pr_codec {
    use prost::Message;

    use super::{MsgBuffer, MsgBufferReader};
    use crate::error::{Error, Result};

    #[inline]
    pub fn ser<T: Message>(t: &T, buf: &mut MsgBuffer) -> Result<()> {
        let cap = t.encoded_len();
        if cap <= buf.get_max_data_size() {
            buf.resize(cap);
            let start = buf.get_inner_buf();
            let len = buf.get_data_size();
            let mut s = unsafe { std::slice::from_raw_parts_mut(start, len) };
            t.encode(&mut s)?;
            Ok(())
        } else {
            Err(Error::Codec(
                format!("message is too large: {cap} > {}", buf.get_max_data_size()).into(),
            ))
        }
    }

    #[inline]
    pub fn de<T: Message + Default>(mut reader: MsgBufferReader) -> Result<T> {
        use bytes::buf::Buf;
        reader.advance(0);
        T::decode(reader).map_err(Into::into)
    }
}
