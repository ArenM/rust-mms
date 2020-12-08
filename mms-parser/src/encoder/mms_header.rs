use super::*;
use crate::types::{
    mms_header, mms_header::ClassIdentifier, MmsHeader, MmsHeaderValue,
};

use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) enum EncodeError {
    HeaderMsg((&'static str, MmsHeader)),
    Msg(&'static str),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HeaderMsg((msg, field)) => {
                write!(f, "{} on field {:?}", msg, field)
            }
            Self::Msg(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for EncodeError {}

impl std::convert::From<(&'static str, MmsHeader)> for EncodeError {
    fn from(e: (&'static str, MmsHeader)) -> Self {
        Self::HeaderMsg(e)
    }
}

impl std::convert::From<&'static str> for EncodeError {
    fn from(e: &'static str) -> Self {
        Self::Msg(e)
    }
}

macro_rules! encode_header_field_builder {
    ($($field_name:ident as $type:ident => $encode:expr),+$(,)*) => {
        #[allow(unused)]
        pub(crate) fn encode_header_field(field: MmsHeader, value: MmsHeaderValue) -> Result<Vec<u8>, EncodeError> {
            let bytes = match field.clone() {
                $(
                    MmsHeader::$field_name => {
                        let mut header_bytes: Vec<u8> = field.clone().into();
                        let mut value_bytes = match value {
                            MmsHeaderValue::$type(v) => {
                                let encoded: Result<Vec<u8>, EncodeError> = $encode(v);
                                Ok(encoded?)
                            },
                            MmsHeaderValue::Bytes(b) => {
                                Ok(b.to_vec())
                            },
                            o => Err(("Wrong value type", field.clone()))

                        }?;
                        header_bytes.append(&mut value_bytes);
                        Ok(header_bytes)
                    }
                )*
                    field => {
                        Err(( "No known encoder", field ))
                    }
            }?;
            Ok(bytes)
        }
    }
}

encode_header_field_builder! {
    // TODO: The following fields are required in order to encode a send request
    // Cc, Bcc
    // ContentType
    XMmsMessageType as MessageTypeField => |v: crate::types::mms_header::MessageTypeField| Ok(encode_byte(v.into())),
    XMmsTransactionId as String => |v| Ok(encode_string(v)),
    XMmsMMSVersion as ShortUint => |v| Ok(encode_short_integer(v)?),
    XMmsDeliveryReport as Bool => |v| Ok(vec![encode_bool(v)]),
    XMmsReadReport as Bool => |v| Ok(vec![encode_bool(v)]),
    XMmsMessageClass as ClassIdentifier => |v: crate::types::mms_header::ClassIdentifier| Ok(
        // TODO: Move move this block somewhere else
        match v {
            ClassIdentifier::Personal => vec![128],
            ClassIdentifier::Advertisment => vec![129],
            ClassIdentifier::Informational => vec![130],
            ClassIdentifier::Auto => vec![131],
            // Technically this should be token-text but the only difference is
            // that token text disallows a few characters
            ClassIdentifier::Other(s) => encode_string(s)
        }
    ),
    From as FromField => |v| Ok(
        match v {
            mms_header::FromField::Address(addr) => encode_address(addr),
            mms_header::FromField::InsertAddress => value_length(vec![129])
        }
    ),
    To as String => |v| Ok(encode_string(v)),
    Subject as String => |v| Ok(encode_string(v)),
    ContentType as ContentType => |v| Ok(encode_content_type(v)),
}
