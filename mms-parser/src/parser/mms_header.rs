use crate::parser::*;
use crate::types::mms_header::MmsHeader::*;
use crate::types::mms_header::*;

use log::debug;
use nom::{bytes::complete::take, IResult};
use mime::Mime;
use std::convert::TryFrom;

pub fn parse_enum_class(d: &[u8]) -> IResult<&[u8], ClassIdentifier> {
    let (d, class) = take(1u8)(d)?;

    let class = match class[0] {
        128 => ClassIdentifier::Personal,
        129 => ClassIdentifier::Advertisment,
        130 => ClassIdentifier::Informational,
        131 => ClassIdentifier::Auto,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                d,
                nom::error::ErrorKind::Satisfy,
            )))
        }
    };

    Ok((d, class))
}

pub fn parse_string_class(d: &[u8]) -> IResult<&[u8], ClassIdentifier> {
    let (d, class) = nom::bytes::complete::take_till1(|c| c == '\u{0}' as u8)(d)?;
    let class = crate::helpers::u8_to_string(class).unwrap();

    Ok((d, ClassIdentifier::Other(class)))
}

macro_rules! parse_header_field_builder {
    ($($field_name:ident as $type:ty => $parse:expr),+$(,)*) => {
        pub(crate) fn parse_header_field(field: MmsHeader, d: &[u8]) -> IResult<&[u8], MmsHeaderValue> {
            match field {
                $(
                    $field_name => {
                        let (d, value): (&[u8], $type) = $parse(d)?;
                        let value = MmsHeaderValue::from(value);
                        Ok((d, value))
                    }
                )*
                    field => {
                        // TODO: This doesn't need to panic, just return an
                        // error
                        // nom::error::Error::from_external_error is probably what is needed for
                        // this
                        debug!("A parser for field `{:?}` isn't implemented yet, falling back to bytes", field);
                        let value = MmsHeaderValue::from(d.to_vec());
                        Ok((&[], value))
                    }
            }
        }
    }
}
// NOTE: I haven't properly tested parsers which are commented out, they should work, but I'd like
// to check first
parse_header_field_builder!{
    // TODO: I haven't been able to properly parse content-type yet
    ContentType as Mime => |d| parse_content_type(d),
    Date as LongUint => |d| parse_long_integer(d),
    From as String => |d| {
        let (d, len) = parse_value_length(d)?;
        let (d, value) = take(len)(d)?;

        let (data, token) = take(1u8)(value)?;
        let token = token[0];

        match token {
            128 => Ok((
                    d,
                    parse_encoded_string_value(data)?.1,
            )),
            129 => unimplemented!(),
            _ => {
                //error
                panic!("Unexpected Token: {:?}", token);
            }
        }
    },
    MessageID as String => |d| parse_text_string(d),
    Subject as String => |d| parse_encoded_string_value(d),
    To as String => |d| parse_encoded_string_value(d),
    //XMmsAdaptationAllowed, Bool => |d| -> IResult<&[u8], bool> {
    //    let (d, allowed) = take(1u8)(d)?;
    //    match allowed[0] {
    //        128 => Ok((d, true)),
    //        129 => Ok((d, false)),
    //        // TODO: Have a recoverable error type, which means that an unknown value was parsed,
    //        // but it is okay to keep going
    //        // Yes when testing this on a mms message reviced on t-mobile there was a value of 115
    //        // which I don't know how to interpret, so I'm just ignoreing it
    //        _ => Ok((d, false))
    //            // _ => unimplemented!()
    //    }
    //},
    XMmsContentLocation as String => |d| parse_text_string(d),
    XMmsDeliveryReport as Bool => |d| -> IResult<&[u8], bool> {
        // TODO: parse bool logic seems to be duplicated, perhaps write a macro for this match?
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    },
    //XMmsExpiry, ExpiryField => |d| {
    //    let (d, len) = parse_value_length(d)?;
    //    let (d, value) = take(len)(d)?;

    //    let (value, token) = take(1u8)(value)?;

    //    let field = match token[0] {
    //        128 => {
    //            let (_, unix_time) = parse_long_integer(value)?;
    //            ExpiryField::Absolute(unix_time)
    //        }
    //        129 => {
    //            let (_, time_delta) = parse_long_integer(value)?;
    //            ExpiryField::Relative(time_delta)
    //        }
    //        _ => {
    //            // TODO: Not valid, return an error
    //            unimplemented!()
    //        }
    //    };

    //    Ok((d, field))
    //},
    //XMmsLimit, LongUint => |d| parse_integer_value(d),
    XMmsMMSVersion as ShortUint => |d| parse_short_integer(d),
    XMmsMessageClass as ClassIdentifier => |d| nom::branch::alt((parse_enum_class, parse_string_class))(d),
    XMmsMessageSize as LongUint => |d| parse_long_integer(d),
    XMmsMessageType as MessageTypeField => |d| -> IResult<&[u8], MessageTypeField> {
        let (d, message_type) = take(1u8)(d)?;
        Ok((
                d,
                // TODO: return error insetad of unwraping
                MessageTypeField::try_from(message_type[0]).unwrap(),
        ))
    },
    XMmsPriority as ShortUint => |d| -> IResult<&[u8], u8> { // TODO: Use enum instead of u8
                let (d, priority) = take(1u8)(d)?;
                let priority = match priority[0] {
                    128 => 1,
                    129 => 2,
                    130 => 3,
                    _ => unimplemented!()
                };
                Ok((d, priority))
    },
    XMmsReadReport as Bool => |d| -> IResult<&[u8], bool> {
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    },
    XMmsRetrieveStatus as RetrieveStatusField => |d| -> IResult<&[u8], RetrieveStatusField> {
        // TODO: Move this to a try_from function
        // Also a seemingly successful get on 
        let (d, status) = take(1u8)(d)?;

        let status = match status[0] {
            128 => RetrieveStatusField::Ok,
            192 => RetrieveStatusField::ErrorTransientFailure,
            193 => RetrieveStatusField::ErrorTransientMessageNotFound,
            194 => RetrieveStatusField::ErrorTransientNetworkProblem,
            195..=223 => RetrieveStatusField::ErrorTransientFailureOther(status[0]),
            224 => RetrieveStatusField::ErrorPermanentFailure,
            225 => RetrieveStatusField::ErrorPermanentServceDenied,
            226 => RetrieveStatusField::ErrorPermanentMessageNotFound,
            227 => RetrieveStatusField::ErrorPermanentContentUnsupported,
            // TODO: This should pontentially be for just 228..255
            _ => RetrieveStatusField::ErrorPermanentFailureOther(status[0]),
        };
        Ok((d, status))
    },
    XMmsTransactionId as String => |d| parse_text_string(d),
    //ImplicitBody, Vec<u8> => |d: &[u8]| -> IResult<&[u8], Vec<u8>> { Ok(( &[], d.to_vec() )) },
}
