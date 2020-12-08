use super::*;
use crate::{
    pdu::take_field,
    types::multipart::{MultiPartBody, MultiPartItem},
    wap_header_item,
};

use nom::{combinator::all_consuming, do_parse, multi::many0, named};

named!(
    body_item<MultiPartItem>,
    do_parse!(
        headers_len: uintvar
            >> data_len: uintvar
            >> content_type_bytes: take_field
            >> headers: take!(headers_len - content_type_bytes.len() as u64)
            >> body: take!(data_len)
            >> (MultiPartItem {
                content_type: all_consuming(parse_content_type)(
                    content_type_bytes
                )?
                .1,
                headers: all_consuming(many0(wap_header_item))(headers)?.1,
                body: body.to_vec(),
            })
    )
);

// TODO: In WAP 1.3 the num_entries (nEntries in the spec) header becomes
// optional, but recommended, so there could be either 2, or 3 uintvars at the
// beginning of the body
pub fn parse_multipart_body(data: &[u8]) -> IResult<&[u8], MultiPartBody> {
    let (mut data, _num_entries) = uintvar(data)?;
    let mut items = Vec::new();

    while data.len() > 0 {
        let (d, item) = body_item(data)?;
        items.push(item);
        data = d;
    }

    Ok((data, items))
}
