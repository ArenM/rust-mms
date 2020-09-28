mod helpers;
mod parser;
mod types;

#[macro_use]
extern crate nom;

use helpers::{null_delimited, u8_to_string};
use parser::{header_item, read_uintvar};
use types::{MessageHeader, PduType, Wap};

use nom::{
    bytes::complete::take, combinator::complete, do_parse, multi::many0, named,
    number::complete::be_u8, IResult,
};

named!(pub parse_data<Wap>,
    do_parse!(
        // TODO: This should ONLY be red in "connectionless PDUs"
        transaction_id: be_u8 >>
        message_type: be_u8 >>
        message_headers: parse_message >>
        (Wap {
                transaction_id,
                message_type: PduType::from(message_type),
                content_type: message_headers.0,
                headers: message_headers.1,
        })
    )
);

pub fn parse_message(d: &[u8]) -> IResult<&[u8], (String, Vec<MessageHeader>)> {
    let (d, header_length) = read_uintvar(d)?;
    let (d, header_content) = take(header_length)(d)?;
    let (_, (content_type, headers)) = complete(message_header)(header_content)?;

    Ok((d, (u8_to_string(content_type), headers)))
}

fn message_header(d: &[u8]) -> IResult<&[u8], (&[u8], Vec<MessageHeader>)> {
    // TODO: This is very case specific, fix it
    let (d, _short_content_type) = take(2u8)(d)?;
    let (d, content_type) = null_delimited(d)?;
    let (d, headers) = many0(header_item)(d)?;

    Ok((d, (content_type, headers)))
}
