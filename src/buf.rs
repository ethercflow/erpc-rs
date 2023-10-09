// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use erpc_sys::erpc::MsgBuffer as RawMsgBuffer;
use std::io::{self, BufRead, Read};

#[repr(C)]
pub struct MsgBufferReader {
    buf: *const RawMsgBuffer,
    offset: usize,
    remain: usize,
}

impl MsgBufferReader {
    pub unsafe fn new(buf: *const RawMsgBuffer) -> Self {
        MsgBufferReader {
            buf,
            offset: 0,
            remain: unsafe { (*buf).get_data_size() },
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.remain
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.remain == 0
    }
}

impl Read for MsgBufferReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.fill_buf()?.read(buf)?;
        self.consume(read);
        Ok(read)
    }
}

impl BufRead for MsgBufferReader {
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.is_empty() {
            return Ok(&[]);
        }
        unsafe {
            let start = (*self.buf).get_inner_buf();
            let len = (*self.buf).get_data_size();
            let s = std::slice::from_raw_parts(start, len);
            Ok(s.get_unchecked(self.offset..))
        }
    }

    fn consume(&mut self, mut amt: usize) {
        if amt > self.remain {
            amt = self.remain;
        }
        self.remain -= amt;
        self.offset += amt;
    }
}

unsafe impl Sync for MsgBufferReader {}
unsafe impl Send for MsgBufferReader {}

impl bytes::Buf for MsgBufferReader {
    fn remaining(&self) -> usize {
        self.remain
    }

    fn chunk(&self) -> &[u8] {
        if self.is_empty() {
            return &[];
        }
        unsafe {
            let start = (*self.buf).get_inner_buf();
            let len = (*self.buf).get_data_size();
            let s = std::slice::from_raw_parts(start, len);
            s.get_unchecked(self.offset..)
        }
    }

    fn advance(&mut self, cnt: usize) {
        self.consume(cnt);
    }
}
