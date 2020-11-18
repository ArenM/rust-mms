use super::*;
use crate::types::{MmsHeader, MmsHeaderValue};

use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) enum EncodeError {
    HeaderMsg((&'static str, MmsHeader)),
    Msg(&'static str),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HeaderMsg((msg, field)) => write!(f, "{} on field {:?}", msg, field),
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
    From as String => |v| Ok(encode_address(v)),
    To as String => |v| Ok(encode_address(v)),
    ContentType as ContentType => |v| Ok(encode_content_type(v)),
}
