pub mod content_type_codes;
pub mod message_header;
pub mod mms_header;
pub mod multipart;

pub use message_header::*;
pub use mms_header::{MmsHeaderValue, MmsHeader};
use PduType::*;

use crate::MultiMap;

// use enum_primitive_derive::Primitive;
// use num_enum::IntoPrimitive;
//
#[derive(Derivative)]
#[derivative(Debug)]
pub struct FetchResponse {
    pub transaction_id: u8,
    pub message_type: PduType,
    pub content_type: String,
    pub headers: Vec<MessageHeader>,
    #[derivative(Debug="ignore")]
    pub data: Vec<u8>,
}

// TODO: Some of these fields might not apply to all wap messages,
// make this more generic
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Wap {
    pub transaction_id: u8,
    pub message_type: PduType,
    pub content_type: mime::Mime,
    pub headers: Vec<MessageHeader>,
    #[derivative(Debug="ignore")]
    pub data: Vec<u8>,
}

// TODO: Move the numbers in this match to the enum definition
// and write From<u8>, and possibly Into<u8> with a macro
impl From<u8> for PduType {
    fn from(t: u8) -> Self {
        match t {
            1 => Connect,
            2 => ConnectReply,
            3 => Redirect,
            4 => Reply,
            5 => Disconnect,
            6 => Push,
            7 => ConfirmedPush,
            8 => Suspend,
            9 => Resume,
            // unassigned block
            64 => Get,
            65 => Options,
            66 => Head,
            67 => Delete,
            68 => Trace,
            // unassigned block
            96 => Post,
            97 => Put,
            // unassigned block
            128 => DataFragment,
            _ => Unknown(t),
        }
    }
}

#[derive(Debug)]
// TODO: This needs a better name
// TODO: use getter methods instead of pub values?
pub struct VndWapMmsMessage {
    pub headers: MultiMap<MmsHeader, MmsHeaderValue>,
    pub body: Vec<u8>,
}

impl VndWapMmsMessage {
    pub fn new(headers: MultiMap<MmsHeader, MmsHeaderValue>) -> Self {
        Self {
            headers,
            body: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            headers: MultiMap::new(),
            body: Vec::new(),
        }
    }
}

#[derive(Debug)]
// TODO: More descriptive name, pdu stands for protocal data unit
pub enum PduType {
    Connect,
    ConnectReply,
    Redirect,
    Reply,
    Disconnect,
    Push,
    ConfirmedPush,
    Suspend,
    Resume,
    Get,
    Options,
    Head,
    Delete,
    Trace,
    Post,
    Put,
    DataFragment,
    Unknown(u8),
}
