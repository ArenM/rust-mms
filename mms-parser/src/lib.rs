mod special_types;

use special_types::uintvar;

use nom::number::complete::be_u8;
use nom::IResult;

// TODO: Move PduType definition to a separate module for better name spacing
use PduType::*;

#[derive(Debug)]
pub struct Wap {
    transaction_id: u8,
    message_type: PduType,
    body: PushMessageBody,
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

pub fn parse_data(data: &[u8]) -> IResult<&[u8], Wap> {
    // TODO: This should ONLY be red in "connectionless PDUs"
    let (r, tid) = be_u8(data)?;

    // TODO: Convert this to an enum based on table 34 in assigned numbers from
    // wap-230-wsp-20010705-a.pdf
    let (r, message_type) = be_u8(r)?;
    let (r, body) = parse_message_body(r, PduType::from(message_type))?;

    Ok((
        r,
        Wap {
            transaction_id: tid,
            message_type: PduType::from(message_type),
            body
        },
    ))
}

#[derive(Debug)]
struct PushMessageBody {
    header_length: usize,
    next_byte: usize,
}

// TODO: PushMessageBody is all I care about for now, but this should be fixed
fn parse_message_body(d: &[u8], _message_type: PduType) -> IResult<&[u8], PushMessageBody> {
    let (d, header_length) = uintvar(d)?;
    let (d, next_byte) = uintvar(d)?;
    Ok((d, PushMessageBody { header_length, next_byte }))
}
