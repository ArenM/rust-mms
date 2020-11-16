use std::convert::TryFrom;

use super::VndWapMmsMessage;

pub(crate) type ShortUint = u8;
pub(crate) type LongUint = u64;
pub(crate) type Bool = bool;
pub(crate) type ContentType = mime::Mime;

// TODO: parse all variants so this isn't necessary
#[derive(Debug, Clone)]
pub enum MmsHeaderValue {
    Bool(bool),
    LongUint(u64),
    ShortUint(u8),
    String(String),
    Bytes(Vec<u8>),
    ContentType(mime::Mime),
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
    };
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

// TODO: Generalize this
macro_rules! header_fields {
    ($name:ident, $(($camel_name:ident, $under_name:ident, $type:ident, $binary_code:expr));+$(;)*) => {
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

        impl Into<Vec<u8>> for $name {
            fn into(self) -> Vec<u8> {
                match self {
                    $(
                        Self::$camel_name => vec![$binary_code],
                    )+
                        Self::UnknownInt(i) => vec![i],
                        Self::ImplicitBody => Vec::new(),
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
    // (Bcc);
    // (Cc);
    // (Content);
    (ContentType, content_type, ContentType, 0x04);
    (Date, date, LongUint, 0x05);
    (From, from, String, 0x09);
    (MessageID, message_id, String, 0x0B);
    // (Mode);
    (Subject, subject, String, 0x16);
    (To, to, String, 0x17);
    (XMmsAdaptationAllowed, x_mms_adaptation_alowed, Bool, 0x3C);
    // (XMmsApplicID);
    // (XMmsAttributes);
    // (XMmsAuxApplicInfo);
    // (XMmsCancelID);
    // (XMmsCancelStatus);
    // (XMmsContentClass);
    (XMmsContentLocation, x_mms_content_location, String, 0x03);
    // (XMmsDRMContent);
    (XMmsDeliveryReport, x_mms_delivery_report, Bool, 0x06);
    // (XMmsDeliveryTime);
    // (XMmsDistributionIndicator);
    // (XMmsElementDescriptor);
    (XMmsExpiry, x_mms_expiry, ExpiryField, 0x08);
    (XMmsLimit, x_mms_limit, LongUint, 0x33);
    // (XMmsMMFlags);
    (XMmsMMSVersion, x_mms_mms_version, ShortUint, 0x0D);
    // (XMmsMMState);
    // (XMmsMboxQuotas);
    // (XMmsMboxTotals);
    (XMmsMessageClass, x_mms_message_class, ClassIdentifier, 0x0A);
    // (XMmsMessageCount);
    (XMmsMessageSize, x_mms_message_size, LongUint, 0x0E);
    // I don't know why the compiler insists that this one needs type annotations
    (XMmsMessageType, x_mms_message_type, MessageTypeField, 0x0C);
    // (XMmsPreviouslySentBy);
    // (XMmsPreviouslySentDate);
    (XMmsPriority, x_mms_priority, ShortUint, 0x0F);
    // (XMmsQuotas);
    (XMmsReadReport, x_mms_read_report, Bool, 0x10);
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
    (XMmsRetrieveStatus, x_mms_retrieve_status, RetrieveStatusField, 0x19);
    // (XMmsRetrieveText);
    // (XMmsSenderVisibility);
    // (XMmsStart);
    // (XMmsStatus);
    // (XMmsStore);
    // (XMmsStoreStatus);
    // (XMmsStoreStatusText);
    // (XMmsStored);
    // (XMmsTotals);
    (XMmsTransactionId, x_mms_transaction_id, String, 0x18)
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
