mod message_header;
mod uintvar;

pub use message_header::*;
pub use uintvar::*;

use nom::{
    bytes::complete::{tag, take, take_till},
    IResult,
};


#[macro_export]
macro_rules! header_fields {
    ($n:ident, $(($e:ident, $bin:expr, $parse:expr));+$(;)*) => {
        #[derive(Debug, Hash, PartialEq, Eq)]
        pub enum $n {
            $(
                $e,
            )+
        }

        pub fn parse_header_item(d: &[u8]) -> IResult<&[u8], (MmsHeader, MmsHeaderValue)> {
            // TODO: I think this can be a string, handle that case
            let (d, header_byte) = take(1u8)(d)?;
            let header_byte = header_byte[0] & 0x7F;

            match header_byte {
                $(
                    $bin => {
                        let (d, value ) = $parse(d)?;
                        let value = MmsHeaderValue::from(value);
                        Ok((d,( $n::$e, value )))
                    }
                )+
                    b => {
                        unimplemented!("No known variant for type {:#04X}", b);
                    }
            }
        }
    }
}

pub fn parse_text_string(d: &[u8]) -> IResult<&[u8], String> {
    let (d, val) = take_till(|c| c == '\u{0}' as u8)(d)?;
    let (d, _) = tag("\u{0}")(d)?;

    if val[0] >= 128 {
        return Err(nom::Err::Error(nom::error::Error::new(
            &[],
            nom::error::ErrorKind::Satisfy,
        )));
    }

    let val = if val[0] == '"' as u8 && val[1] >= 128 {
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

pub fn parse_short_integer(d: &[u8]) -> IResult<&[u8], u8> {
    let (r, bit) = take(1u8)(d)?;
    let bit = bit[0];

    if bit & 0x80 == 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            &[],
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

    // This is copied from tally_u7_nums in uintvar.rs, it may be a good idea to
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

pub fn parse_value_length(d: &[u8]) -> IResult<&[u8], u64> {
    let (d, l1) = take(1u8)(d)?;

    match l1[0] {
        0..=30 => Ok((d, l1[0] as u64)),
        31 => read_uintvar(d),
        _ => Err(nom::Err::Error(nom::error::Error::new(
            &[],
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
    nom::branch::alt((parse_text_string, parse_value_length_charset_string))(d)
}

#[cfg(test)]
mod test {
    use super::*;

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
        let (_, val) = parse_text_string("\"something\u{0}".as_bytes()).unwrap();

        assert_eq!(val, "\"something");
    }

    #[test]
    fn quoted_text_string() {
        let (_, val1) = parse_text_string("\"\u{128}etc...\u{0}".as_bytes()).unwrap();
        let (_, val2) = parse_text_string("\"\u{255}etc...\u{0}".as_bytes()).unwrap();

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
