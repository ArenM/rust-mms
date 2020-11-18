use super::*;

pub(crate) fn encode_content_type(content_type: mime::Mime) -> Vec<u8> {
    // Simpliest possible encoding which should be valid, this will get a lot
    // more complicated in order to support multipart messages
    encode_string(content_type.essence_str().to_string())
}
