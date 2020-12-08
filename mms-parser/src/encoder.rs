mod content_type;
pub(crate) mod mms_header;
use content_type::*;

use crate::types::VndWapMmsMessage;
use std::convert::TryFrom;

pub fn encode_mms_message(message: VndWapMmsMessage) -> Vec<u8> {
    let mut message = message;
    let mut encoded = Vec::new();

    // TODO: Enforce the following:
    // "In the encoding of the header fields, the order of the fields is not
    // significant, except that X-Mms-Message-Type, X-Mms-Transaction-ID (when
    // present) and X-Mms-MMS-Version MUST be at the beginning of the message
    // headers, in that order, and if the PDU contains a message body the
    // Content Type MUST be the last header field, followed by message body."

    let encoded_headers: Vec<Vec<u8>> = message
        .headers
        .iter()
        .map(|header| {
            mms_header::encode_header_field(header.0.clone(), header.1.clone())
                .unwrap()
        })
        .collect();

    encoded.append(&mut encoded_headers.concat());
    encoded.append(&mut message.body);

    encoded
}

fn encode_byte(b: u8) -> Vec<u8> {
    vec![b]
}

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
        panic!("Encoding uintvars isn't supported yet")
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
