mod encoder;
mod helpers;
mod parser;
mod pdu;
pub mod types;

pub use pdu::*;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate derivative;

use parser::{header_item, parse_content_type, uintvar};
use types::{MessageHeader, PduType, VndWapMmsMessage, Wap};

use nom::{
    bytes::complete::take, combinator::complete, do_parse, named, number::complete::be_u8, IResult,
};

impl Wap {
    // TODO: Replace Option with Result
    pub fn parse_body(&self) -> Option<VndWapMmsMessage> {
        match self.content_type.essence_str() {
            "application/vnd.wap.mms-message" => {
                let ctx = ParserCtx {
                    message_class: MessageClass { has_body: false },
                };
                let split = match split_header_fields(&*self.data, ctx) {
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

fn parse_message_headers(d: &[u8]) -> IResult<&[u8], (mime::Mime, Vec<MessageHeader>)> {
    let (d, header_length) = uintvar(d)?;
    let (d, header_content) = take(header_length)(d)?;
    let (_, (content_type, headers)) = complete(message_headers)(header_content)?;

    Ok((d, (content_type, headers)))
}

fn content_type(d: &[u8]) -> IResult<&[u8], mime::Mime> {
    nom::combinator::map_parser(pdu::take_field, parse_content_type)(d)
}

// TODO: this should return a content type struct or a string rather than a &[u8]
named!(
    message_headers<(mime::Mime, Vec<MessageHeader>)>,
    do_parse!(
        take!(0)
            >> content_type: content_type
            >> headers: many0!(header_item)
            >> (content_type, headers)
    )
);
