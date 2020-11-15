use crate::types::{MmsHeader, MmsHeaderValue};
use super::*;

pub(crate) enum EncodeError {
    HeaderMsg((String, MmsHeader)),
}

impl std::convert::From<(String, MmsHeader)> for EncodeError {
    fn from(e: (String, MmsHeader)) -> Self {
        Self::HeaderMsg(e)
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
                            o => Err((format!("Wrong value type {:?}", o), field.clone()))

                        }?;
                        header_bytes.append(&mut value_bytes);
                        Ok(header_bytes)
                    }
                )*
                    field => {
                        Err(( "A encoder for field `{:?}` isn't implemented yet".to_string(), field ))
                    }
            }?;
            Ok(bytes)
        }
    }
}

encode_header_field_builder! {
    MessageID as String => |v| {
        Ok(encode_string(v))
    },
}
