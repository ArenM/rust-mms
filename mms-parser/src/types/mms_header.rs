use std::convert::TryFrom;

use super::{ContentType, VndWapMmsMessage};
use crate::parser::*;
use nom::{bytes::complete::take, IResult};
use self::MmsHeader::*;

type ShortUint = u8;
type LongUint = u64;
type Bool = bool;

// TODO: parse all variants so this isn't necessary
#[derive(Debug, Clone)]
pub enum MmsHeaderValue {
    Bool(bool),
    LongUint(u64),
    ShortUint(u8),
    String(String),
    Bytes(Vec<u8>),
    ContentType(ContentType),
    ExpiryField(ExpiryField),
    ClassIdentifier(ClassIdentifier),
    MessageTypeField(MessageTypeField),
    RetrieveStatusField(RetrieveStatusField),
}

macro_rules! mms_header_from {
    ($variant:ident, $type:ty) => {
        impl std::convert::From<$type> for MmsHeaderValue {
            fn from(value: $type) -> Self {
                Self::$variant(value)
            }
        }
    }
}

mms_header_from!(Bool, bool);
mms_header_from!(LongUint, u64);
mms_header_from!(ShortUint, u8);
mms_header_from!(String, String);
mms_header_from!(Bytes, Vec<u8>);
mms_header_from!(ContentType, ContentType);
mms_header_from!(ExpiryField, ExpiryField);
mms_header_from!(ClassIdentifier, ClassIdentifier);
mms_header_from!(MessageTypeField, MessageTypeField);
mms_header_from!(RetrieveStatusField, RetrieveStatusField);

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
    ($($field_name:ident, $type:ty => $parse:expr),+$(,)*) => {
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
                        unimplemented!("A parser for field `{:?}` isn't implemented yet", field);
                    }
            }
        }
    }
}

parse_header_field_builder!{
    ContentType, ContentType => |d| pase_content_type(d),
    Date, LongUint => |d| parse_long_integer(d),
    From, String => |d| {
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
    MessageID, String => |d| parse_text_string(d),
    Subject, String => |d| parse_encoded_string_value(d),
    To, String => |d| parse_encoded_string_value(d),
    XMmsAdaptationAllowed, Bool => |d| -> IResult<&[u8], bool> {
        let (d, allowed) = take(1u8)(d)?;
        match allowed[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            // TODO: Have a recoverable error type, which means that an unknown value was parsed,
            // but it is okay to keep going
            // Yes when testing this on a mms message reviced on t-mobile there was a value of 115
            // which I don't know how to interpret, so I'm just ignoreing it
            _ => Ok((d, false))
                // _ => unimplemented!()
        }
    },
    XMmsContentLocation, String => |d| parse_text_string(d),
    XMmsDeliveryReport, Bool => |d| -> IResult<&[u8], bool> {
        // TODO: parse bool logic seems to be duplicated, perhaps write a macro for this match?
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    },
    XMmsExpiry, ExpiryField => |d| {
        let (d, len) = parse_value_length(d)?;
        let (d, value) = take(len)(d)?;

        let (value, token) = take(1u8)(value)?;

        let field = match token[0] {
            128 => {
                let (_, unix_time) = parse_long_integer(value)?;
                ExpiryField::Absolute(unix_time)
            }
            129 => {
                let (_, time_delta) = parse_long_integer(value)?;
                ExpiryField::Relative(time_delta)
            }
            _ => {
                // TODO: Not valid, return an error
                unimplemented!()
            }
        };

        Ok((d, field))
    },
    XMmsLimit, LongUint => |d| parse_integer_value(d),
    XMmsMMSVersion, ShortUint => |d| parse_short_integer(d),
    XMmsMessageClass, ClassIdentifier => |d| nom::branch::alt((parse_enum_class, parse_string_class))(d),
    XMmsMessageSize, LongUint => |d| parse_long_integer(d),
    XMmsMessageType, MessageTypeField => |d| -> IResult<&[u8], MessageTypeField> {
        let (d, message_type) = take(1u8)(d)?;
        Ok((
                d,
                // TODO: return error insetad of unwraping
                MessageTypeField::try_from(message_type[0]).unwrap(),
        ))
    },
    XMmsPriority, ShortUint => |d| -> IResult<&[u8], u8> { // TODO: Use enum instead of u8
                let (d, priority) = take(1u8)(d)?;
                let priority = match priority[0] {
                    128 => 1,
                    129 => 2,
                    130 => 3,
                    _ => unimplemented!()
                };
                Ok((d, priority))
    },
    XMmsReadReport, Bool => |d| -> IResult<&[u8], bool> {
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    },
    XMmsRetrieveStatus, RetrieveStatusField => |d| -> IResult<&[u8], RetrieveStatusField> {
        let (d, status) = take(1u8)(d)?;

        let status = match status[0] {
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
    XMmsTransactionId, String => |d| parse_text_string(d),
    ImplicitBody, Vec<u8> => |d: &[u8]| -> IResult<&[u8], Vec<u8>> { Ok(( &[], d.to_vec() )) },
}


// TODO: Generalize this
macro_rules! header_fields {
    ($name:ident, $(($camel_name:ident, $under_name:ident, $type:ident, $binary_code:expr, $parse:expr));+$(;)*) => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone)]
        pub enum $name {
            $(
                $camel_name,
            )+
                UnknownInt(u8),
                ImplicitBody,
        }

        impl std::convert::From<u8> for $name {
            fn from(d: u8) -> Self {
                match d {
                    $(
                        $binary_code => $name::$camel_name,
                    )+
                        v => $name::UnknownInt(v)
                }
            }
        }

        // TODO: This should probably eithre be TryInto or return a &[u8]
        impl Into<u8> for $name {
            fn into(self) -> u8 {
                match self {
                    $(
                        Self::$camel_name => $binary_code,
                    )+
                        Self::UnknownInt(i) => i,
                        // TODO: This isn't correct, and will produce an invalid
                        // message if used to encode
                        Self::ImplicitBody => 0,
                }
            }
        }

        pub fn parse_header_item(d: &[u8]) -> IResult<&[u8], (MmsHeader, MmsHeaderValue)> {
            // TODO: I think this can be a string, handle that case
            let (d, header_byte) = take(1u8)(d)?;
            if header_byte[0] & 0x80 == 0 {
                // TODO: do something better here
                panic!("{:#04X} doesn't have it's 8th bit set to 1", header_byte[0])
            }
            let header_byte = header_byte[0] & 0x7F;
            println!("Header Byte: {:#04X}", header_byte);

            match header_byte {
                $(
                    $binary_code => {
                        // println!("Matched header {:#04X}", $binary_code);
                        let (d, value): (&[u8], $type) = $parse(d)?;
                        let value = MmsHeaderValue::$type(value);
                        Ok((d,( $name::$camel_name, value )))
                    }
                )+
                    b => {
                        // TODO: it is possible to safely take the header data without parsing the
                        // header see wap-230 8.4.1.2
                        unimplemented!("No known variant for type {:#04X}", b);
                    }
            }
        }

        impl VndWapMmsMessage {
            $(
                pub fn $under_name(&self) -> Option<&$type> {
                    match self.headers.get(&$name::$camel_name) {
                        Some(v) => match v {
                            MmsHeaderValue::$type(d) => Some(d),
                            u => panic!("Unexpected value in $camel_name: {:?}", u)
                        },
                        None => None
                    }
                }
            )+
        }
    }
}

// TODO: It may be necessary to have a unknown field for encoding messages
header_fields! {
    MmsHeader,
    // (Additionalheaders);
    // (Bcc);
    // (Cc);
    // (Content);
    (ContentType, content_type, ContentType, 0x04, |d| pase_content_type(d));
    (Date, date, LongUint, 0x05, |d| parse_long_integer(d));
    (From, from, String, 0x09, |d| {
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
    });
    (MessageID, message_id, String, 0x0B, |d| parse_text_string(d));
    // (Mode);
    (Subject, subject, String, 0x16, |d| parse_encoded_string_value(d));
    (To, to, String, 0x17, |d| parse_encoded_string_value(d));
    (XMmsAdaptationAllowed, x_mms_adaptation_alowed, Bool, 0x3C, |d| -> IResult<&[u8], bool> {
        let (d, allowed) = take(1u8)(d)?;
        match allowed[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            // TODO: Have a recoverable error type, which means that an unknown value was parsed,
            // but it is okay to keep going
            // Yes when testing this on a mms message reviced on t-mobile there was a value of 115
            // which I don't know how to interpret, so I'm just ignoreing it
            _ => Ok((d, false))
            // _ => unimplemented!()
        }
    });
    // (XMmsApplicID);
    // (XMmsAttributes);
    // (XMmsAuxApplicInfo);
    // (XMmsCancelID);
    // (XMmsCancelStatus);
    // (XMmsContentClass);
    (XMmsContentLocation, x_mms_content_location, String, 0x03, |d| parse_text_string(d));
    // (XMmsDRMContent);
    (XMmsDeliveryReport, x_mms_delivery_report, Bool, 0x06, |d| -> IResult<&[u8], bool> {
        // TODO: parse bool logic seems to be duplicated, perhaps write a macro for this match?
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    });
    // (XMmsDeliveryTime);
    // (XMmsDistributionIndicator);
    // (XMmsElementDescriptor);
    (XMmsExpiry, x_mms_expiry, ExpiryField, 0x08, |d| {
        let (d, len) = parse_value_length(d)?;
        let (d, value) = take(len)(d)?;

        let (value, token) = take(1u8)(value)?;

        let field = match token[0] {
            128 => {
                let (_, unix_time) = parse_long_integer(value)?;
                ExpiryField::Absolute(unix_time)
            }
            129 => {
                let (_, time_delta) = parse_long_integer(value)?;
                ExpiryField::Relative(time_delta)
            }
            _ => {
                // TODO: Not valid, return an error
                unimplemented!()
            }
        };

        Ok((d, field))
    });
    (XMmsLimit, x_mms_limit, LongUint, 0x33, |d| parse_integer_value(d));
    // (XMmsMMFlags);
    (XMmsMMSVersion, x_mms_mms_version, ShortUint, 0x0D, |d| parse_short_integer(d));
    // (XMmsMMState);
    // (XMmsMboxQuotas);
    // (XMmsMboxTotals);
    (XMmsMessageClass, x_mms_message_class, ClassIdentifier, 0x0A, |d| nom::branch::alt((parse_enum_class, parse_string_class))(d));
    // (XMmsMessageCount);
    (XMmsMessageSize, x_mms_message_size, LongUint, 0x0E, |d| parse_long_integer(d));
    // I don't know why the compiler insists that this one needs type annotations
    (XMmsMessageType, x_mms_message_type, MessageTypeField, 0x0C, |d| -> IResult<&[u8], MessageTypeField> {
        let (d, message_type) = take(1u8)(d)?;
        Ok((
                d,
                // TODO: return error insetad of unwraping
                MessageTypeField::try_from(message_type[0]).unwrap(),
        ))
    });
    // (XMmsPreviouslySentBy);
    // (XMmsPreviouslySentDate);
    (XMmsPriority, x_mms_priority, ShortUint, 0x0F, |d| -> IResult<&[u8], u8> { // TODO: Use enum instead of u8
        let (d, priority) = take(1u8)(d)?;
        let priority = match priority[0] {
            128 => 1,
            129 => 2,
            130 => 3,
            _ => unimplemented!()
        };
        Ok((d, priority))
    });
    // (XMmsQuotas);
    (XMmsReadReport, x_mms_read_report, Bool, 0x10, |d| -> IResult<&[u8], Bool> {
        let (d, report) = take(1u8)(d)?;
        match report[0] {
            128 => Ok((d, true)),
            129 => Ok((d, false)),
            _ => unimplemented!() // TODO: just return an error
        }
    });
    // (XMmsReadStatus);
    // (XMmsRecommendedRe);
    // (XMmsRecommendedRetrieval);
    // (XMmsReplaceID);
    // (XMmsReplyApplicID);
    // (XMmsReplyCharging);
    // (XMmsReplyChargingDeadline);
    // (XMmsReplyChargingID);
    // (XMmsReplyChargingSize);
    // (XMmsReportAllowed);
    // (XMmsResponseStatus);
    // (XMmsResponseText);
    (XMmsRetrieveStatus, x_mms_retrieve_status, RetrieveStatusField, 0x19,
     |d| -> IResult<&[u8], RetrieveStatusField>{
        let (d, status) = take(1u8)(d)?;

        let status = match status[0] {
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
    });
    // (XMmsRetrieveText);
    // (XMmsSenderVisibility);
    // (XMmsStart);
    // (XMmsStatus);
    // (XMmsStore);
    // (XMmsStoreStatus);
    // (XMmsStoreStatusText);
    // (XMmsStored);
    // (XMmsTotals);
    (XMmsTransactionId, x_mms_transaction_id, String, 0x18, |d| parse_text_string(d))
}

#[derive(Debug, Clone)]
pub enum ClassIdentifier {
    Personal,
    Advertisment,
    Informational,
    Auto,
    Other(String),
}

// TODO: use date time and time deltas for this (probably from chrono crate)
// or even better convert realitve time to absolute when parsing, and just store that
#[derive(Debug, Clone)]
pub enum ExpiryField {
    Absolute(u64),
    Relative(u64),
}

#[derive(Debug, Clone)]
pub enum MessageTypeField {
    MSendReq,
    MSendConf,
    MNotificationInd,
    MNotifyrespInd,
    MRetrieveConf,
    MAcknowledgeInd,
    MDeliveryInd,
    MReadRecInd,
    MReadOrigInd,
    MForwardReq,
    MForwardConf,
    MMboxStoreReq,
    MMboxStoreConf,
    MMboxViewReq,
    MMboxViewConf,
    MMboxUploadReq,
    MMboxUploadConf,
    MMboxDeleteReq,
    MMboxDeleteConf,
    MMboxDescr,
    MDeleteReq,
    MDeleteConf,
    MCancelReq,
    MCancelConf,
}

#[derive(Debug, Clone)]
pub enum RetrieveStatusField {
    Ok,
    ErrorTransientFailure,
    ErrorTransientFailureOther(u8),
    ErrorTransientMessageNotFound,
    ErrorTransientNetworkProblem,
    ErrorPermanentFailure,
    ErrorPermanentFailureOther(u8),
    ErrorPermanentServceDenied,
    ErrorPermanentMessageNotFound,
    ErrorPermanentContentUnsupported,
}

impl TryFrom<u8> for MessageTypeField {
    type Error = &'static str;

    fn try_from(i: u8) -> Result<Self, &'static str> {
        match i {
            128 => Ok(MessageTypeField::MSendReq),
            129 => Ok(MessageTypeField::MSendConf),
            130 => Ok(MessageTypeField::MNotificationInd),
            131 => Ok(MessageTypeField::MNotifyrespInd),
            132 => Ok(MessageTypeField::MRetrieveConf),
            133 => Ok(MessageTypeField::MAcknowledgeInd),
            134 => Ok(MessageTypeField::MDeliveryInd),
            135 => Ok(MessageTypeField::MReadRecInd),
            136 => Ok(MessageTypeField::MReadOrigInd),
            137 => Ok(MessageTypeField::MForwardReq),
            138 => Ok(MessageTypeField::MForwardConf),
            139 => Ok(MessageTypeField::MMboxStoreReq),
            140 => Ok(MessageTypeField::MMboxStoreConf),
            141 => Ok(MessageTypeField::MMboxViewReq),
            142 => Ok(MessageTypeField::MMboxViewConf),
            143 => Ok(MessageTypeField::MMboxUploadReq),
            144 => Ok(MessageTypeField::MMboxUploadConf),
            145 => Ok(MessageTypeField::MMboxDeleteReq),
            146 => Ok(MessageTypeField::MMboxDeleteConf),
            147 => Ok(MessageTypeField::MMboxDescr),
            148 => Ok(MessageTypeField::MDeleteReq),
            149 => Ok(MessageTypeField::MDeleteConf),
            150 => Ok(MessageTypeField::MCancelReq),
            151 => Ok(MessageTypeField::MCancelConf),
            _ => Err("Unknown value for X-Mms-Message-Type"),
        }
    }
}
