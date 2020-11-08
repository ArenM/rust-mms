use crate::header_fields;
use std::convert::TryFrom;

use super::VndWapMmsMessage;
use crate::parser::*;
use nom::{bytes::complete::take, IResult};

type ShortUint = u8;
type LongUint = u64;

// TODO: parse all variants so this isn't necessary
#[derive(Debug)]
pub enum MmsHeaderValue {
    LongUint(u64),
    ShortUint(u8),
    String(String),
    ExpiryField(ExpiryField),
    ClassIdentifier(ClassIdentifier),
    MessageTypeField(MessageTypeField),
}

impl From<String> for MmsHeaderValue {
    fn from(d: String) -> Self {
        Self::String(d)
    }
}

impl From<ExpiryField> for MmsHeaderValue {
    fn from(d: ExpiryField) -> Self {
        Self::ExpiryField(d)
    }
}

impl From<u64> for MmsHeaderValue {
    fn from(d: u64) -> Self {
        Self::LongUint(d)
    }
}

impl From<u8> for MmsHeaderValue {
    fn from(d: u8) -> Self {
        Self::ShortUint(d)
    }
}

impl From<ClassIdentifier> for MmsHeaderValue {
    fn from(d: ClassIdentifier) -> Self {
        Self::ClassIdentifier(d)
    }
}

impl From<MessageTypeField> for MmsHeaderValue {
    fn from(d: MessageTypeField) -> Self {
        Self::MessageTypeField(d)
    }
}

pub fn parse_enum_class(d: &[u8]) -> IResult<&[u8], ClassIdentifier> {
    let (d, class) = take(1u8)(d)?;

    let class = match class[0] {
        128 => ClassIdentifier::Personal,
        129 => ClassIdentifier::Advertisment,
        130 => ClassIdentifier::Informational,
        131 => ClassIdentifier::Auto,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                &[],
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

// TODO: It may be necessary to have a unknown field for encoding messages
header_fields! {
    MmsHeader,
    // (Additionalheaders);
    // (Bcc);
    // (Cc);
    // (Content);
    // (ContentType);
    // (Date);
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
    // (MessageID);
    // (Mode);
    // (Subject);
    // (To);
    // (XMmsAdaptationAllowed);
    // (XMmsApplicID);
    // (XMmsAttributes);
    // (XMmsAuxApplicInfo);
    // (XMmsCancelID);
    // (XMmsCancelStatus);
    // (XMmsContentClass);
    (XMmsContentLocation, x_mms_content_location, String, 0x03, |d| parse_text_string(d));
    // (XMmsDRMContent);
    // (XMmsDeliveryReport);
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
    // (XMmsLimit);
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
    // (XMmsPriority);
    // (XMmsQuotas);
    // (XMmsReadReport);
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
    // (XMmsRetrieveStatus);
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

#[derive(Debug)]
pub enum ClassIdentifier {
    Personal,
    Advertisment,
    Informational,
    Auto,
    Other(String),
}

// TODO: use date time and time deltas for this (probably from chrono crate)
// or even better convert realitve time to absolute when parsing, and just store that
#[derive(Debug)]
pub enum ExpiryField {
    Absolute(u64),
    Relative(u64),
}

#[derive(Debug)]
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
