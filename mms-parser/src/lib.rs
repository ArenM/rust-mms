mod helpers;
mod parser;
pub mod types;
pub mod pdu;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate derivative;

use helpers::{take_till_null, u8_to_string};
use parser::{header_item, uintvar};
use types::{
    MessageHeader, MmsHeader, MmsHeaderValue, PduType, VndWapMmsMessage, Wap,
};

use multimap::MultiMap;
use nom::{
    bytes::complete::take, combinator::complete, do_parse, multi::many1, named,
    number::complete::be_u8, IResult,
};

pub fn parse_mms_pdu(data: &[u8]) -> IResult<&[u8], VndWapMmsMessage > {
    println!("Mms Pdu First Byte: {:#04X}", data[0]);
    let (data, mut headers) = complete(many1(types::mms_header::parse_header_item))(data)?;
    let headers: MultiMap<MmsHeader, MmsHeaderValue> = headers.drain(..).collect();
    Ok((data, VndWapMmsMessage::new(headers)))
}

impl Wap {
    // TODO: Replace Option with Result
    pub fn parse_body(&self) -> Option<VndWapMmsMessage> {
        match &*self.content_type {
            "application/vnd.wap.mms-message" => {
                match parse_mms_pdu(&self.data) {
                    Ok(d) => Some(d.1),
                    Err(_) => None
                }
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

fn parse_message_headers(d: &[u8]) -> IResult<&[u8], (String, Vec<MessageHeader>)> {
    let (d, header_length) = uintvar(d)?;
    let (d, header_content) = take(header_length)(d)?;
    let (_, (content_type, headers)) = complete(message_headers)(header_content)?;

    Ok((d, (u8_to_string(content_type).unwrap(), headers)))
}

// TODO: this should return a content type struct or a string rather than a &[u8]
named!(
    message_headers<(&[u8], Vec<MessageHeader>)>,
    do_parse!(
        take!(2)
            >> content_type: take_till_null
            >> headers: many0!(header_item)
            >> (content_type, headers)
    )
);
