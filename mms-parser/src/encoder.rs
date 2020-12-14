mod content_type;
pub(crate) mod mms_header;
pub mod multipart;

use content_type::*;
use multipart::EncodableBody;

use crate::{
    types::{
        mms_header::{MmsHeader, MmsHeaderValue},
        MessageHeader,
    },
    MultiMap,
};

use std::convert::TryFrom;

pub fn encode_mms_message(
    headers: MultiMap<MmsHeader, MmsHeaderValue>,
    body: impl EncodableBody,
) -> Vec<u8> {
    let mut encoded = Vec::new();

    if let Some(_) = headers.get(&MmsHeader::ContentType) {
        // TODO: Remove need to panic
        panic!("Headers must not contain a content_type");
    }

    // TODO: Enforce the following:
    // "In the encoding of the header fields, the order of the fields is not
    // significant, except that X-Mms-Message-Type, X-Mms-Transaction-ID (when
    // present) and X-Mms-MMS-Version MUST be at the beginning of the message
    // headers, in that order, and if the PDU contains a message body the
    // Content Type MUST be the last header field, followed by message body."
    let encoded_headers: Vec<Vec<u8>> = headers
        .iter()
        .map(|header| {
            mms_header::encode_header_field(header.0.clone(), header.1.clone())
                .unwrap()
        })
        .collect();

    encoded.append(&mut encoded_headers.concat());
    encoded.append(
        &mut mms_header::encode_header_field(
            MmsHeader::ContentType,
            body.content_type().clone().into(),
        )
        .unwrap(),
    );
    encoded.append(&mut body.encode());

    encoded
}

fn encode_wap_headers(headers: Vec<MessageHeader>) -> Vec<u8> {
    let mut buf = Vec::new();
    use crate::types::message_header::MessageHeader::*;

    for header in headers {
        match header {
            ContentLocation(v) => {
                buf.push(0x0E | 0x80);
                buf.append(&mut encode_string(v));
            }
            ContentId(v) => {
                buf.push(0x40 | 0x80);
                buf.append(&mut encode_quoted_string(v));
            }
            b => panic!("Unable to encode {:?}, no known encoder", b),
        }
    }

    buf
}

pub(crate) fn encode_uintvar(num: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut num = num;

    while num > 0 {
        let n = u8::try_from(num & 0x7F).unwrap();
        num = num >> 7;
        buf.push(n | 0x80);
    }

    buf.reverse();
    let last = buf.pop().unwrap_or(0);
    buf.push(last & 0x7F);

    buf
}

fn encode_byte(b: u8) -> Vec<u8> {
    // TODO: Should this set the first bit to 1?
    vec![b]
}

fn encode_quoted_string(v: String) -> Vec<u8> {
    let mut buf = encode_string(v);

    if let Some(n) = buf.get(1) {
        if n > &0x7F {
            panic!("Invalid character in quoted string");
        }
    }

    buf.insert(0, '"' as u8);
    buf
}

// TODO: There are multiple string types with different allowed characters, add
// mor functions which check supported characters
fn encode_string(v: String) -> Vec<u8> {
    let mut bytes: Vec<u8> = v.into_bytes().to_vec();
    bytes.push(0);
    if !(32..=127).contains(&bytes[0]) {
        bytes.insert(0, 127);
    }
    bytes
}

fn encode_bool(v: bool) -> u8 {
    match v {
        true => 128,
        false => 129,
    }
}

fn encode_short_integer(v: u8) -> Result<Vec<u8>, &'static str> {
    if v > 0x7F {
        return Err("Integer to large to encode as a short-integer");
    }

    Ok(vec![v | 0x80])
}

fn encode_value_length(len: u64) -> Vec<u8> {
    if len <= 30 {
        vec![u8::try_from(len).unwrap()]
    } else {
        let mut buf = vec![31];
        buf.append(&mut encode_uintvar(len));
        buf
    }
}

fn value_length(mut v: Vec<u8>) -> Vec<u8> {
    let len = v.len();
    let mut len_bytes = encode_value_length(len as u64);
    len_bytes.append(&mut v);
    len_bytes
}

fn encode_address(v: String) -> Vec<u8> {
    let mut inner = encode_string(v);
    let mut outer = vec![128];
    outer.append(&mut inner);
    value_length(outer)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn encode_1byte_uintvar() {
        assert_eq!(vec![123u8], encode_uintvar(123))
    }

    #[test]
    fn encode_2byte_uintvar() {
        assert_eq!(
            vec![0b10000101, 0b00000001],
            encode_uintvar(0b1010000001u64)
        )
    }

    #[test]
    fn encode_multi_byte_uintvar() {
        assert_eq!(
            vec![0b10000001, 0b10000000, 0b10000000, 0b00000011],
            encode_uintvar(0b1000000000000000000011u64)
        );
    }
}
