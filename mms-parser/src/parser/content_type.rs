use super::*;
use mime::Mime;
use nom::{combinator::map, IResult};

// TODO: This will probably need to be shortened with a macro, there are about
// 30 of these
fn parse_well_known_parameter(d: &[u8]) -> IResult<&[u8], String> {
    let (d, param) = parse_integer_value(d)?;
    // From wap-230-wsp table 38
    match param {
        0x01 => {
            // TODO: implement proper well-known-charset parsing
            if d[0] == 128 {
                Ok((&d[1..], "charset=\"*\"".to_string()))
            } else {
                let (d, chr_set) = parse_integer_value(d)?;
                Ok((d, format!("charset=\"{}\"", chr_set)))
            }
        }
        0x09 => {
            let (d, value) = parse_constrained_encoding(d)?;
            Ok((d, format!("type=\"{}\"", value)))
        }
        0x0A => {
            let (d, value) = parse_text_string(d)?;
            Ok((d, format!("start=\"{}\"", value)))
        }
        i => unimplemented!("{}", i),
    }
}

// TODO: This needs a macro as well
fn parse_well_known_content_type(d: &[u8]) -> IResult<&[u8], String> {
    let (d, m) = parse_short_integer(d)?;
    // from https://www.openmobilealliance.org/wp/OMNA/wsp/wsp_content_type_codes.html
    let m = match m {
        0x33 => "application/vnd.wap.multipart.related".to_string(),
        _ => format!("mms-well-known/{}", m),
    };

    Ok((d, m))
}

fn parse_constrained_encoding(d: &[u8]) -> IResult<&[u8], String> {
    let (d, v) = alt((
        map(parse_short_integer, |i| format!("mms-well-known/{}", i)),
        parse_text_string,
    ))(d)?;

    Ok((d, v))
}

fn parse_content_type_general_form(d: &[u8]) -> IResult<&[u8], String> {
    let (d, len) = parse_value_length(d)?;
    let (d, header) = take(len)(d)?;

    let (mut params_data, media) = match header[0] >= 128 {
        true => parse_well_known_content_type(header),
        false => parse_text_string(header),
    }?;

    let mut params = Vec::new();

    while params_data.len() > 0 {
        match parse_integer_value(params_data) {
            Ok((r, p)) => {
                let (r, param) = parse_well_known_parameter(params_data)?;
                params_data = r;
                params.push(param);
            }
            Err(_) => unimplemented!(),
        }
    }

    let ct = if params.len() >= 1 {
        let params = params.join("; ");
        [media, params].join("; ")
    } else {
        media
    };

    Ok((d, ct))
}

// see wap-230-wsp-20010705-a.pdf section 8.4.2.24
pub fn parse_content_type(d: &[u8]) -> IResult<&[u8], Mime> {
    let (d, c) = match d[0] {
        0..=31 => parse_content_type_general_form(d),
        32..=255 => parse_constrained_encoding(d),
    }?;

    println!("content type: {}", c);
    let mime_type: Mime = c.parse().unwrap();
    Ok((&[], mime_type))
}
