mod content_type;
pub(crate) mod mms_header;
pub mod multipart;

use content_type::*;
use multipart::EncodableBody;

use crate::{
    types::{
        mms_header::{self as mms_header_types, MmsHeader, MmsHeaderValue},
        MessageHeader,
    },
    MultiMap,
};

use std::{
    convert::TryFrom,
    fs::File,
    io::Read,
    ops::{Deref, DerefMut},
    path::Path,
};

pub struct MSendReq {
    headers: MultiMap<MmsHeader, MmsHeaderValue>,
    body: multipart::EncoderBuilder<multipart::RelatedBodyPart>,
}

impl MSendReq {
    pub fn new() -> Self {
        Self {
            headers: MultiMap::new(),
            body: multipart::EncoderBuilder::new(),
        }
    }
    fn finalize_headers(&mut self) {
        use mms_header_types::MessageTypeField;
        use MmsHeader::*;

        let mut headers = vec![
            (XMmsMessageType, MessageTypeField::MSendReq.into()),
            (
                XMmsTransactionId,
                self.headers
                    .remove(&XMmsTransactionId)
                    // TODO: This may be dangerous behaviour, there is nowhere the id can be
                    // easily returned
                    .unwrap_or(uuid::Uuid::new_v4().to_string().into()),
            ),
            (XMmsMMSVersion, crate::MMS_VERSION.into()),
        ];

        // TODO: This should check for dupliate headers, espicilly of the headers listed above
        let mut user_headers: Vec<(MmsHeader, MmsHeaderValue)> = self
            .headers
            .drain_pairs()
            .map(|(key, values)| {
                // TODO: Only include specifc keys and limit most of them to
                // only one value per key
                values
                    .map(|v| (key.clone(), v))
                    .collect::<Vec<(MmsHeader, MmsHeaderValue)>>()
            })
            .flatten()
            .collect();

        headers.append(&mut user_headers);
        let headers: MultiMap<_, _> = headers.drain(..).collect();

        self.headers = headers;
    }
    // TODO: Most of these functions should return an error if there is already a value set
    pub fn transaction_id(&mut self, id: String) {
        self.insert(MmsHeader::XMmsTransactionId, id.into());
    }
    pub fn from(&mut self, addr: mms_header_types::FromField) {
        self.insert(MmsHeader::From, addr.into());
    }
    // TODO: Replace Strings in to, cc and bcc with an address enum of some sort
    pub fn to(&mut self, addr: String) {
        self.insert(MmsHeader::To, addr.into());
    }
    pub fn cc(&mut self, addr: String) {
        self.insert(MmsHeader::Cc, addr.into());
    }
    pub fn bcc(&mut self, addr: String) {
        self.insert(MmsHeader::Bcc, addr.into());
    }
    pub fn subject(&mut self, subject: String) {
        self.insert(MmsHeader::Subject, subject.into());
    }
    pub fn class(&mut self, class: mms_header_types::ClassIdentifier) {
        self.insert(MmsHeader::XMmsMessageClass, class.into());
    }
    pub fn delivery_report(&mut self, report: bool) {
        self.insert(MmsHeader::XMmsDeliveryReport, report.into());
    }
    pub fn read_report(&mut self, report: bool) {
        self.insert(MmsHeader::XMmsReadReport, report.into());
    }
    // TODO: Proper error handling
    pub fn body_part(&mut self, part: multipart::RelatedBodyPart) {
        self.body.part(part)
    }
    pub fn body_file<P: AsRef<Path>>(&mut self, file: P) {
        let file = file.as_ref();
        let mut mime = mime_from_file(file).to_string();

        let data = {
            let mut file = File::open(file).unwrap();
            let mut buffer: Vec<u8> = Vec::new();

            file.read_to_end(&mut buffer).unwrap();
            buffer
        };
        let id = file_id(&file);
        let location = file_name(&file);

        mime.push_str(&*format!("; name=\"{}\"", location));
        let mime = mime.parse().unwrap();

        let item = multipart::RelatedBodyPart::new(
            mime,
            data,
            format!("<{}>", id),
            location.to_string(),
        );

        self.body_part(item)
    }
    pub fn encode(mut self) -> Vec<u8> {
        self.finalize_headers();
        let complete_body = self.body.build().unwrap();
        encode_mms_message(self.headers, complete_body)
    }
}

fn mime_from_file<P: AsRef<Path>>(file: P) -> mime::Mime {
    let file = file.as_ref();
    let error_message =
        |file| format!("Couldn't determine content type for file {:?}", file);

    let extension: &str = file
        .extension()
        .expect(&*error_message(file))
        .to_str()
        .expect(&*error_message(file));

    // TODO: This is really hacky because I get the wrong content type back from mime_db
    if extension == "smil" {
        "application/smil".parse().unwrap()
    } else {
        mime_db::lookup(extension)
            .expect(&*error_message(file))
            .parse()
            .expect(&*error_message(file))
    }
}

fn file_id<P: AsRef<Path>>(file: P) -> String {
    let id = file
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    id
}

fn file_name<P: AsRef<Path>>(file: P) -> String {
    let location = file
        .as_ref()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    location
}

impl Deref for MSendReq {
    type Target = MultiMap<MmsHeader, MmsHeaderValue>;

    fn deref(&self) -> &Self::Target {
        &self.headers
    }
}

impl DerefMut for MSendReq {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers
    }
}

pub fn encode_mms_message(
    headers: MultiMap<MmsHeader, MmsHeaderValue>,
    body: impl EncodableBody,
) -> Vec<u8> {
    let mut encoded = Vec::new();

    if let Some(_) = headers.get(&MmsHeader::ContentType) {
        // TODO: Remove need to panic
        panic!("Headers must not contain a content_type");
    }

    // TODO: Enforce the following:
    // "In the encoding of the header fields, the order of the fields is not
    // significant, except that X-Mms-Message-Type, X-Mms-Transaction-ID (when
    // present) and X-Mms-MMS-Version MUST be at the beginning of the message
    // headers, in that order, and if the PDU contains a message body the
    // Content Type MUST be the last header field, followed by message body."
    let encoded_headers: Vec<Vec<u8>> = headers
        .iter()
        .map(|header| {
            mms_header::encode_header_field(header.0.clone(), header.1.clone())
                .unwrap()
        })
        .collect();

    encoded.append(&mut encoded_headers.concat());
    encoded.append(
        &mut mms_header::encode_header_field(
            MmsHeader::ContentType,
            body.content_type().clone().into(),
        )
        .unwrap(),
    );
    encoded.append(&mut body.encode());

    encoded
}

fn encode_wap_headers(headers: Vec<MessageHeader>) -> Vec<u8> {
    let mut buf = Vec::new();
    use crate::types::message_header::MessageHeader::*;

    for header in headers {
        match header {
            ContentLocation(v) => {
                buf.push(0x0E | 0x80);
                buf.append(&mut encode_string(v));
            }
            ContentId(v) => {
                buf.push(0x40 | 0x80);
                buf.append(&mut encode_quoted_string(v));
            }
            b => panic!("Unable to encode {:?}, no known encoder", b),
        }
    }

    buf
}

pub(crate) fn encode_uintvar(num: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut num = num;

    while num > 0 {
        let n = u8::try_from(num & 0x7F).unwrap();
        num = num >> 7;
        buf.push(n | 0x80);
    }

    buf.reverse();
    let last = buf.pop().unwrap_or(0);
    buf.push(last & 0x7F);

    buf
}

fn encode_byte(b: u8) -> Vec<u8> {
    // TODO: Should this set the first bit to 1?
    vec![b]
}

fn encode_quoted_string(v: String) -> Vec<u8> {
    let mut buf = encode_string(v);

    if let Some(n) = buf.get(1) {
        if n > &0x7F {
            panic!("Invalid character in quoted string");
        }
    }

    buf.insert(0, '"' as u8);
    buf
}

// TODO: There are multiple string types with different allowed characters, add
// mor functions which check supported characters
fn encode_string(v: String) -> Vec<u8> {
    let mut bytes: Vec<u8> = v.into_bytes().to_vec();
    bytes.push(0);
    if !(32..=127).contains(&bytes[0]) {
        bytes.insert(0, 127);
    }
    bytes
}

fn encode_bool(v: bool) -> u8 {
    match v {
        true => 128,
        false => 129,
    }
}

fn encode_short_integer(v: u8) -> Result<Vec<u8>, &'static str> {
    if v > 0x7F {
        return Err("Integer to large to encode as a short-integer");
    }

    Ok(vec![v | 0x80])
}

fn encode_value_length(len: u64) -> Vec<u8> {
    if len <= 30 {
        vec![u8::try_from(len).unwrap()]
    } else {
        let mut buf = vec![31];
        buf.append(&mut encode_uintvar(len));
        buf
    }
}

fn value_length(mut v: Vec<u8>) -> Vec<u8> {
    let len = v.len();
    let mut len_bytes = encode_value_length(len as u64);
    len_bytes.append(&mut v);
    len_bytes
}

fn encode_address(v: String) -> Vec<u8> {
    let mut inner = encode_string(v);
    let mut outer = vec![128];
    outer.append(&mut inner);
    value_length(outer)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn encode_1byte_uintvar() {
        assert_eq!(vec![123u8], encode_uintvar(123))
    }

    #[test]
    fn encode_2byte_uintvar() {
        assert_eq!(
            vec![0b10000101, 0b00000001],
            encode_uintvar(0b1010000001u64)
        )
    }

    #[test]
    fn encode_multi_byte_uintvar() {
        assert_eq!(
            vec![0b10000001, 0b10000000, 0b10000000, 0b00000011],
            encode_uintvar(0b1000000000000000000011u64)
        );
    }
}
