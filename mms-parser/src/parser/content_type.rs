use super::*;
use crate::types::content_type_codes::CONTENT_TYPE_CODES;
use mime::Mime;
use nom::{combinator::map, IResult};

fn well_known_charset(chr_set: u64) -> Option<String> {
    // From http://www.iana.org/assignments/character-sets/character-sets.xhtml
    Some(
        match chr_set {
            3 => "US-ASCII",
            4 => "ISO-8859-1",
            106 => "UTF-8",
            _ => return None,
        }
        .to_string(),
    )
}

// TODO: This will probably need to be shortened with a macro, there are about
// 30 of these
fn parse_well_known_parameter(d: &[u8], param: u64) -> IResult<&[u8], String> {
    // From wap-230-wsp table 38
    match param {
        0x01 => {
            if d[0] == 128 {
                Ok((&d[1..], "charset=\"*\"".to_string()))
            } else {
                let (d, chr_set) = parse_integer_value(d)?;
                Ok((
                    d,
                    format!(
                        "charset=\"{}\"",
                        well_known_charset(chr_set)
                            .unwrap_or(format!("{}", chr_set))
                    ),
                ))
            }
        }
        0x05 => {
            let (d, value) = parse_text_string(d)?;
            Ok((d, format!("name=\"{}\"", value)))
        }
        0x09 => {
            let (d, value) = parse_constrained_encoding(d)?;
            Ok((d, format!("type=\"{}\"", value)))
        }
        0x0A => {
            let (d, value) = parse_text_string(d)?;
            Ok((d, format!("start=\"{}\"", value)))
        }
        i => unimplemented!(
            "Cannot parse well known parameter {:#04X} in content_type",
            i
        ),
    }
}

fn parse_well_known_content_type(d: &[u8]) -> IResult<&[u8], String> {
    let (d, m) = parse_short_integer(d)?;
    let m = CONTENT_TYPE_CODES
        .iter()
        .find(|i| i.0 == m)
        .map(|i| i.1.to_string())
        .unwrap_or(format!("mms-unknown/{}", m));

    Ok((d, m))
}

fn parse_constrained_encoding(d: &[u8]) -> IResult<&[u8], String> {
    let (d, v) = alt((parse_well_known_content_type, parse_text_string))(d)?;

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
                let (r, param) = parse_well_known_parameter(r, p)?;
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

    let mime_type: Mime = c.parse().unwrap();
    Ok((&[], mime_type))
}

#[cfg(test)]
mod test {
    use super::*;

    fn mime(mime: &str) -> mime::Mime {
        mime.parse::<mime::Mime>().unwrap()
    }

    #[test]
    fn content_string() {
        let (r, c) = parse_content_type("text/plain".as_bytes()).unwrap();

        assert_eq!(r, &[]);
        assert_eq!(c, mime("text/plain"));
    }

    #[test]
    fn short_int() {
        let (r, c) = parse_content_type(&[0xB3]).unwrap();

        assert_eq!(c, mime("application/vnd.wap.multipart.related"))
    }

    #[test]
    fn general_int() {
        // techically this should fail, but it'll be useful for debugging if one
        // of the tests breaks
        let (r, c) = parse_content_type(&[0x01, 0xB3]).unwrap();

        assert_eq!(r, &[]);
        assert_eq!(c, mime("application/vnd.wap.multipart.related"))
    }

    #[test]
    fn general_int_with_charset() {
        let (r, c) = parse_content_type(&[0x03, 0xB3, 0x81, 0x83]).unwrap();

        assert_eq!(r, &[]);
        assert_eq!(c, "application/vnd.wap.multipart.related; charset=us-ascii")
    }

    #[test]
    fn general_int_with_any_charset() {
        let (r, c) = parse_content_type(&[0x03, 0xB3, 0x81, 0x80]).unwrap();

        assert_eq!(r, &[]);
        assert_eq!(c, "application/vnd.wap.multipart.related; charset=*/*")
    }

    #[test]
    fn general_string() {
        // let (r, c) = parse_content_type(b"\x0Etext/plain\x00\x81\x80").unwrap();
        let (r, c) =
            parse_content_type(b"\x0Funusual/type\x00\x81\x80").unwrap();

        assert_eq!(r, &[]);
        assert_eq!(c, "unusual/type; charset=*/*")
    }
}
