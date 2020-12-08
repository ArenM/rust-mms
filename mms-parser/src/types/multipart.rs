use super::message_header::MessageHeader;
use mime::Mime;

pub type MultiPartBody = Vec<MultiPartItem>;

#[derive(Debug, Clone)]
pub struct MultiPartItem {
    pub content_type: Mime,
    pub headers: Vec<MessageHeader>,
    pub body: Vec<u8>,
}
