use crate::types::{MessageHeader, MessageHeader::*};
use log::error;
use nom::{bytes::complete::take, IResult};

pub fn header_item(d: &[u8]) -> IResult<&[u8], MessageHeader> {
    // This can be a string, handle that case
    let (d, header_byte) = take(1u8)(d)?;
    let header_byte = header_byte[0] & 0x7F;

    let (d, header_item) = match header_byte {
        // Accept Charset
        0x01 => {
            let (d, c) = take(1u8)(d)?;
            (d, AcceptCharset(c[0]))
        },
        // Content Length
        0x0D => {
            let (d, l) = take(1u8)(d)?;
            (d, ContentLength(l[0] as usize))
        },
        // XWapApplicationId
        0x2F => {
            let (d, id) = take(1u8)(d)?;
            (d, XWapApplicationId(id[0] as usize))
        },
        b => {
            if cfg!(debug_assertions) {
                unimplemented!("No known variant for type {:#04X}", b);
            } else {
                error!("No known variant for type {:#04X}, Skiping rest of Header", b);
                let d: &[u8] = &[];
                (d, UnknownHeader(b))
            }
        }
    };

    Ok((d, header_item))
}
