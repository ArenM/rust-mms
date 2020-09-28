pub mod message_header;

pub use message_header::*;
use PduType::*;

// use enum_primitive_derive::Primitive;
// use num_enum::IntoPrimitive;

#[derive(Debug)]
pub struct Wap {
    pub transaction_id: u8,
    pub message_type: PduType,
    pub content_type: String,
    pub headers: Vec<MessageHeader>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
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
