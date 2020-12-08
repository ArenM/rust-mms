pub mod encoder;
mod helpers;
mod parser;
mod pdu;
pub mod types;

pub use parser::parse_multipart_body;
pub use pdu::*;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate derivative;

use parser::{header_item, parse_content_type, uintvar};
use types::{MessageHeader, PduType, VndWapMmsMessage, Wap};

use nom::{
    bytes::complete::take,
    combinator::{all_consuming, map_parser},
    do_parse, named,
    number::complete::be_u8,
    IResult,
};

pub(crate) use ordered_multimap::ListOrderedMultimap as MultiMap;

pub const MMS_VERSION: u8 = 18;

impl Wap {
    // TODO: Replace Option with Result
    pub fn parse_body(&self) -> Option<VndWapMmsMessage> {
        match self.content_type.essence_str() {
            "application/vnd.wap.mms-message" => {
                let split = match split_header_fields(&*self.data) {
                    Ok((remainder, s)) => {
                        if remainder.len() > 0 {
                            return None;
                        }
                        s
                    }
                    Err(_) => return None,
                };
                let parsed = parse_header_fields(&split);
                Some(VndWapMmsMessage::new(parsed))
            }
            _ => None,
        }
    }
}

// TODO: Put this somewhere else so I don't have to look at it
named!(pub parse_wap_push<Wap>,
    do_parse!(
        // TODO: This field should ONLY be red in "connectionless PDUs" it could cause problems
        transaction_id: be_u8 >>
        message_type: be_u8 >>
        message_headers: parse_message_headers >>
        data: take_all >>
        (Wap {
                transaction_id,
                message_type: PduType::from(message_type),
                content_type: message_headers.0,
                headers: message_headers.1,
                data,
        })
    )
);

fn take_all(d: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let e: &[u8] = &[];
    Ok((e, d.to_vec()))
}

fn parse_message_headers(
    d: &[u8],
) -> IResult<&[u8], (mime::Mime, Vec<MessageHeader>)> {
    let (d, header_length) = uintvar(d)?;
    let (d, header_content) = take(header_length)(d)?;
    let (_, (content_type, headers)) =
        all_consuming(message_headers)(header_content)?;

    Ok((d, (content_type, headers)))
}

fn content_type(d: &[u8]) -> IResult<&[u8], mime::Mime> {
    map_parser(pdu::take_field, parse_content_type)(d)
}

pub fn wap_header_item(d: &[u8]) -> IResult<&[u8], MessageHeader> {
    // This can be a string, handle that case
    let (d, header_byte) = take(1u8)(d)?;
    let header_byte = header_byte[0] & 0x7F;
    let (d, raw_field) = take_field(d)?;

    let (_, parsed_field) = header_item(header_byte, raw_field)?;

    Ok((d, parsed_field))
}

// TODO: this should return a content type struct or a string rather than a &[u8]
named!(
    pub message_headers<(mime::Mime, Vec<MessageHeader>)>,
    do_parse!(
        take!(0)
            >> content_type: content_type
            >> headers: many0!(wap_header_item)
            >> (content_type, headers)
    )
);
