#![allow(unused)]

use crate::parser::*;
use crate::types::mms_header::{MmsHeader, MmsHeaderValue};

use multimap::MultiMap;
use nom::{bytes::complete::take, IResult};

// TODO: Wrap everything here in a Refcell so builder functions don't need to be mut
// #[derive(Default)]
// struct ParserBuilder {
//     message_class: Option<MessageClass>,
//     data: Option<Vec<u8>>,
// }

// TODO: Return a MmsHeader instad of a u8, and handle string headers
fn parse_header_name(d: &[u8]) -> IResult<&[u8], MmsHeader> {
    let (d, header_byte) = take(1u8)(d)?;
    if header_byte[0] & 0x80 == 0 {
        // TODO: No need to panic, just return an error
        panic!("{:#04X} doesn't have it's 8th bit set to 1", header_byte[0])
    }
    let header_byte = header_byte[0] & 0x7F;

    Ok((d, MmsHeader::from(header_byte)))
}

fn take_header_field(d: &[u8]) -> IResult<&[u8], (MmsHeader, Vec<u8>)> {
    let (d, header_byte) = parse_header_name(d)?;

    let (_, first_byte) = take(1u8)(d)?;
    let first_byte = first_byte[0];

    let (d, header_value) = match first_byte {
        0..=30 => take(first_byte + 1)(d),
        31 => {
            let (d, len) = uintvar(d)?;
            take(len)(d)
        }
        32..=127 => take_text_string(d),
        128..=255 => take(1u8)(d),
    }?;

    Ok((d, (header_byte, header_value.to_vec())))
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

    Ok((d, header_fields))
}

pub fn parse_header_fields(
    fields: &Vec<(MmsHeader, Vec<u8>)>,
) -> MultiMap<MmsHeader, MmsHeaderValue> {
    parse_header_fields_with_errors(fields).iter().filter_map(|h| match h.1 {
        Ok(d) => Some((h.0.clone(), d.clone())),
        Err(_) => None
    }).collect()
}

pub fn parse_header_fields_with_errors<'a>(
    fields: &'a Vec<(MmsHeader, Vec<u8>)>,
) -> MultiMap<MmsHeader, Result<MmsHeaderValue, nom::Err<nom::error::Error<&'a [u8]>>>> {
    fields
        .iter()
        .map(|i| {
            let value = match crate::types::mms_header::parse_header_field(i.0.clone(), &*i.1) {
                Ok((r, v)) => v,
                Err(e) => {
                    let err = e;
                    return (i.0.clone(), Err(err))
                },
            };
            (i.0.clone(), Ok(value))
        })
        .collect()
}

pub struct ParserCtx {
    pub message_class: MessageClass,
}

// struct Paresr {
//     message_class: MessageClass,
//     raw_data: Vec<u8>,
//     split_headers: Vec<(MmsHeader, Vec<u8>)>,
//     body: Vec<u8>,
// }

// impl Paresr {
//     pub fn build() -> ParserBuilder {
//         ParserBuilder::default()
//     }

//     pub fn has_body(&self) -> bool {
//         self.message_class.has_body
//     }

//     fn parse(&mut self) -> IResult<&Self, ()> {

//         unimplemented!()
//     }

//     fn take_header(&mut self) -> IResult<&[u8], (&u8, &[u8])> {
//         let (_, header_field) = self.parse_header_field()?;
//         unimplemented!()
//     }

//     fn parse_header_field(&mut self) -> IResult<&[u8], u8> {
//         let (d, header_byte) = take(1u8)(&*self.raw_data)?;
//         if header_byte[0] & 0x80 == 0 {
//             // TODO: do something better here
//             panic!("{:#04X} doesn't have it's 8th bit set to 1", header_byte[0])
//         }
//         Ok((d, header_byte[0] & 0x7F))
//     }
// }

// impl ParserBuilder {
//     fn class(&mut self, class: MessageClass) {
//         self.message_class = Some(class);
//     }

//     fn data(&mut self, data: Vec<u8>) {
//         self.data = Some(data)
//     }

//     fn parse(self) -> Option<Paresr> {
//         let mut parser = Paresr {
//             message_class: self.message_class?,
//             raw_data: self.data?,
//             split_headers: vec![],
//             body: vec![],
//         };
//         parser.parse();
//         Some(parser)
//     }
// }

// TODO: This might need to be a trait to implement getters for required fields
// that don't return a result / option
#[derive(Clone)]
pub struct MessageClass {
    pub has_body: bool,
}
