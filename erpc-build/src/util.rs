// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::str;

// A struct that divide a name into serveral parts that meets rust's guidelines.
struct NameSpliter<'a> {
    name: &'a [u8],
    pos: usize,
}

impl<'a> NameSpliter<'a> {
    fn new(s: &str) -> NameSpliter {
        NameSpliter {
            name: s.as_bytes(),
            pos: 0,
        }
    }
}

impl<'a> Iterator for NameSpliter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.pos == self.name.len() {
            return None;
        }
        // skip all prefix '_'
        while self.pos < self.name.len() && self.name[self.pos] == b'_' {
            self.pos += 1;
        }
        let mut pos = self.name.len();
        let mut upper_len = 0;
        let mut meet_lower = false;
        for i in self.pos..self.name.len() {
            let c = self.name[i];
            if c.is_ascii_uppercase() {
                if meet_lower {
                    // So it should be AaA or aaA
                    pos = i;
                    break;
                }
                upper_len += 1;
            } else if c == b'_' {
                pos = i;
                break;
            } else {
                meet_lower = true;
                if upper_len > 1 {
                    // So it should be AAa
                    pos = i - 1;
                    break;
                }
            }
        }
        let s = str::from_utf8(&self.name[self.pos..pos]).unwrap();
        self.pos = pos;
        Some(s)
    }
}

/// Adjust method name to follow rust-guidelines.
pub fn to_snake_case(name: &str) -> String {
    let mut snake_method_name = String::with_capacity(name.len());
    for s in NameSpliter::new(name) {
        snake_method_name.push_str(&s.to_lowercase());
        snake_method_name.push('_');
    }
    snake_method_name.pop();
    snake_method_name
}

pub fn fq_erpc(item: &str) -> String {
    format!("::erpc_rs::prelude::{item}")
}

#[cfg(test)]
mod test {
    #[test]
    fn test_snake_name() {
        let cases = vec![
            ("AsyncRequest", "async_request"),
            ("asyncRequest", "async_request"),
            ("async_request", "async_request"),
            ("createID", "create_id"),
            ("AsyncRClient", "async_r_client"),
            ("CreateIDForReq", "create_id_for_req"),
            ("Create_ID_For_Req", "create_id_for_req"),
            ("Create_ID_For__Req", "create_id_for_req"),
            ("ID", "id"),
            ("id", "id"),
        ];

        for (origin, exp) in cases {
            let res = super::to_snake_case(origin);
            assert_eq!(res, exp);
        }
    }
}
