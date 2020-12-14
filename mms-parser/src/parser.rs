mod content_type;
mod message_header;
pub(crate) mod mms_header;
mod multipart;
mod uintvar;

pub use content_type::*;
pub use message_header::*;
pub use multipart::parse_multipart_body;
pub use uintvar::*;

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_till1},
    combinator::opt,
    IResult,
};

pub fn take_text_string(d: &[u8]) -> IResult<&[u8], &[u8]> {
    let (dc, val) = take_till(|c| c == 0)(d)?;
    let (dc, _) = tag("\u{0}")(dc)?;
    Ok((dc, &d[..val.len() + 1]))
}

pub fn parse_text_string(d: &[u8]) -> IResult<&[u8], String> {
    let (d, val) = take_till1(|c| c == '\u{0}' as u8)(d)?;

    let (d, _) = opt(tag("\x00"))(d)?;

    if val[0] >= 128 {
        return Err(nom::Err::Error(nom::error::Error::new(
            d,
            nom::error::ErrorKind::Satisfy,
        )));
    }

    let val = if val[0] == 0x7f && val[1] >= 128 {
        &val[1..]
    } else {
        val
    };

    let val = match super::helpers::u8_to_string(val) {
        Ok(v) => Ok(v),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            d,
            nom::error::ErrorKind::Satisfy,
        ))),
    }?;

    Ok((d, val))
}

pub fn parse_quoted_string(d: &[u8]) -> IResult<&[u8], String> {
    let (d, _) = tag("\"")(d)?;
    let (d, v) = take_till1(|c| c == 0)(d)?;
    let (d, _) = opt(tag("\u{0}"))(d)?;

    Ok((d, String::from_utf8_lossy(v).to_string()))
}

pub fn parse_short_integer(d: &[u8]) -> IResult<&[u8], u8> {
    let (r, bit) = take(1u8)(d)?;
    let bit = bit[0];

    if bit & 0x80 == 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            d,
            nom::error::ErrorKind::Satisfy,
        )));
    }

    Ok((r, bit & 0x7F))
}

pub fn parse_long_integer(d: &[u8]) -> IResult<&[u8], u64> {
    let (d, len) = take(1u8)(d)?;
    let len = len[0];

    if len > 30 {
        return Err(nom::Err::Error(nom::error::Error::new(
            &[],
            nom::error::ErrorKind::Satisfy,
        )));
    };

    let (d, bytes) = take(len)(d)?;

    // TODO: This is very similar to tally_u7_nums in uintvar.rs, it may be a good idea to
    // turn it into a seperate function.
    let total = bytes
        .iter()
        .rev()
        .fold((0u64, 0u8), |(acc, iter), x| {
            let x = x.clone() as u64;
            (acc + (x << 8 * iter), iter + 1)
        })
        .0;

    Ok((d, total))
}

fn short_integer_u64(d: &[u8]) -> IResult<&[u8], u64> {
    let (d, i) = parse_short_integer(d)?;
    Ok((d, i as u64))
}

pub fn parse_integer_value(d: &[u8]) -> IResult<&[u8], u64> {
    alt((short_integer_u64, parse_long_integer))(d)
}

pub fn parse_value_length(data: &[u8]) -> IResult<&[u8], u64> {
    let (remainder, l1) = take(1u8)(data)?;

    match l1[0] {
        0..=30 => Ok((remainder, l1[0] as u64)),
        31 => uintvar(remainder),
        _ => Err(nom::Err::Error(nom::error::Error::new(
            data,
            nom::error::ErrorKind::Satisfy,
        ))),
    }
}

fn parse_value_length_charset_string(d: &[u8]) -> IResult<&[u8], String> {
    // TODO: handle charsets, and use the value length, instead of hoping the
    // string ends in null
    let (d, _len) = parse_value_length(d)?;
    let (d, _charset_id) = take(1u8)(d)?;

    parse_text_string(d)
}

pub fn parse_encoded_string_value(d: &[u8]) -> IResult<&[u8], String> {
    alt((parse_text_string, parse_value_length_charset_string))(d)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn take_basic_text_string() {
        let input = "asdf\x00not included".as_bytes();
        let (remainder, val) = take_text_string(input).unwrap();

        assert_eq!(val, "asdf\x00".as_bytes());
        assert_eq!(remainder, "not included".as_bytes());
    }

    #[test]
    fn simple_text_string() {
        let input = "asdf\u{0}not included".as_bytes();
        let (remainder, val) = parse_text_string(input).unwrap();
        let remainder = crate::helpers::u8_to_string(remainder).unwrap();

        assert_eq!(val, "asdf");
        assert_eq!(remainder, "not included");
    }

    #[test]
    fn text_string_with_quote() {
        let (_, val) =
            parse_text_string("\x7fsomething\u{0}".as_bytes()).unwrap();

        assert_eq!(val, "\x7fsomething");
    }

    #[test]
    fn text_string_quoted_value() {
        let (_, val1) =
            parse_text_string("\x7f\u{128}etc...\u{0}".as_bytes()).unwrap();
        let (_, val2) =
            parse_text_string("\x7f\u{255}etc...\u{0}".as_bytes()).unwrap();

        assert_eq!(val1, "\u{128}etc...");
        assert_eq!(val2, "\u{255}etc...");
    }

    #[test]
    fn text_string_should_be_quoted() {
        parse_text_string("\u{128}etc...\u{0}".as_bytes()).unwrap_err();
    }

    #[test]
    fn simple_short_integer() {
        let (_, n) = parse_short_integer(&[0b10000011u8]).unwrap();
        assert_eq!(n, 0b11u8);
    }

    #[test]
    fn invalid_short_integer() {
        parse_short_integer(&[0b00000011]).unwrap_err();
    }

    #[test]
    fn simple_value_length() {
        let (r, l) = parse_value_length(&[22, 42]).unwrap();

        assert_eq!(l, 22);
        assert_eq!(r, &[42]);
    }

    #[test]
    fn uintvar_value_length() {
        let (r, l) = parse_value_length(&[31, 42, 33]).unwrap();

        assert_eq!(l, 42);
        assert_eq!(r, &[33]);
    }

    #[test]
    fn invalid_value_length() {
        parse_value_length(&[32]).unwrap_err();
    }
}
