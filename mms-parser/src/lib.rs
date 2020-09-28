mod helpers;
mod parser;
mod types;

#[macro_use]
extern crate nom;

use helpers::{null_delimited, u8_to_string};
use parser::{header_item, read_uintvar};
use types::{MessageHeader, PduType, Wap};

use nom::{
    bytes::complete::take, combinator::complete, do_parse, named, number::complete::be_u8, IResult,
};

named!(pub parse_data<Wap>,
    do_parse!(
        // TODO: This field should ONLY be red in "connectionless PDUs" it could cause problems
        transaction_id: be_u8 >>
        message_type: be_u8 >>
        message_headers: parse_message >>
        body: take_all >>
        (Wap {
                transaction_id,
                message_type: PduType::from(message_type),
                content_type: message_headers.0,
                headers: message_headers.1,
                body,
        })
    )
);

fn take_all(d: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let e: &[u8] = &[];
    Ok((e, d.to_vec()))
}

fn parse_message(d: &[u8]) -> IResult<&[u8], (String, Vec<MessageHeader>)> {
    let (d, header_length) = read_uintvar(d)?;
    let header_length = if header_length > u32::MAX.into() {
        // TODO: use a softer error here
        panic!("Message too large to process")
    } else {
        header_length.to_u32_digits()[0]
    };
    let (d, header_content) = take(header_length)(d)?;
    let (_, (content_type, headers)) = complete(message_headers)(header_content)?;

    Ok((d, (u8_to_string(content_type), headers)))
}

// TODO: this &[u8] return should chagne to a "content type" type
named!(
    message_headers<(&[u8], Vec<MessageHeader>)>,
    do_parse!(
        take!(2)
            >> content_type: null_delimited
            >> headers: many0!(header_item)
            >> (content_type, headers)
    )
);
