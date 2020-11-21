use crate::types::{MessageHeader, MessageHeader::*};
use nom::{bytes::complete::take, IResult};

pub fn header_item(header_byte: u8, d: &[u8]) -> IResult<&[u8], MessageHeader> {
    let (d, header_item) = match header_byte {
        // Accept Charset
        0x01 => {
            let (d, c) = take(1u8)(d)?;
            (d, AcceptCharset(c[0]))
        }
        // Content Length
        0x0D => {
            let (d, l) = take(1u8)(d)?;
            (d, ContentLength(l[0] as usize))
        }
        // XWapApplicationId
        0x2F => {
            let (d, id) = take(1u8)(d)?;
            (d, XWapApplicationId(id[0] as usize))
        }
        b => (d, UnknownHeader((b, d.to_vec()))),
    };

    Ok((d, header_item))
}
