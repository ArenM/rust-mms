#![allow(unused)]

use crate::parser::*;
use crate::types::mms_header::{MmsHeader, MmsHeaderValue};

use multimap::MultiMap;
use nom::{bytes::complete::take, IResult};

fn parse_header_name(d: &[u8]) -> IResult<&[u8], MmsHeader> {
    let (d, header_byte) = take(1u8)(d)?;
    if header_byte[0] & 0x80 == 0 {
        // TODO: No need to panic, just return an error
        panic!("{:#04X} doesn't have it's 8th bit set to 1", header_byte[0])
    }
    let header_byte = header_byte[0] & 0x7F;

    Ok((d, MmsHeader::from(header_byte)))
}

pub(crate) fn take_header_field(d: &[u8]) -> IResult<&[u8], (MmsHeader, Vec<u8>)> {
    let (d, header_byte) = parse_header_name(d)?;
    let (d, header_value) = take_field(d)?;
    Ok((d, (header_byte, header_value.to_vec())))
}

pub(crate) fn take_field(d: &[u8]) -> IResult<&[u8], &[u8]> {
    let (_, first_byte) = take(1u8)(d)?;
    let first_byte = first_byte[0];

    let (d, header_value) = match first_byte {
        0..=30 => take(first_byte + 1)(d),
        31 => {
            let (pu, len) = uintvar(&d[1..])?;
            take(len + (d.len() as u64 - pu.len() as u64))(d)
        }
        32..=127 => take_text_string(d),
        128..=255 => take(1u8)(d),
    }?;

    Ok((d, header_value))
}

pub fn split_header_fields(d: &[u8], ctx: ParserCtx) -> IResult<&[u8], Vec<(MmsHeader, Vec<u8>)>> {
    let mut header_fields = Vec::new();
    let mut data = d;

    while data.len() > 0 {
        let (d, header) = take_header_field(data)?;
        let header_name = header.0.clone();
        data = d;

        header_fields.push(header);

        // Header side effects
        match header_name {
            MmsHeader::ContentType => {
                if ctx.message_class.has_body {
                    header_fields.push((MmsHeader::ImplicitBody, d.to_vec()));
                    data = &[]
                }
            }
            _ => {}
        }
    }

    Ok((data, header_fields))
}

pub fn parse_header_fields(
    fields: &Vec<(MmsHeader, Vec<u8>)>,
) -> MultiMap<MmsHeader, MmsHeaderValue> {
    parse_header_fields_with_errors(fields)
        .iter()
        .filter_map(|h| match h.1 {
            Ok(d) => Some((h.0.clone(), d.clone())),
            Err(_) => None,
        })
        .collect()
}

pub fn parse_header_fields_with_errors<'a>(
    fields: &'a Vec<(MmsHeader, Vec<u8>)>,
) -> MultiMap<MmsHeader, Result<MmsHeaderValue, nom::Err<nom::error::Error<&'a [u8]>>>> {
    fields
        .iter()
        .map(|i| {
            let value = match crate::parser::mms_header::parse_header_field(i.0.clone(), &*i.1) {
                Ok((r, v)) => v,
                Err(e) => {
                    let err = e;
                    return (i.0.clone(), Err(err));
                }
            };
            (i.0.clone(), Ok(value))
        })
        .collect()
}

#[derive(Clone)]
pub struct ParserCtx {
    pub message_class: MessageClass,
}

// TODO: This is a duplicat of types::mms_header::MessageTypeField we should
// probably just parse that early, add it to the ctx varible, and impliment
// informational functions on it
#[derive(Clone)]
pub struct MessageClass {
    pub has_body: bool,
}
