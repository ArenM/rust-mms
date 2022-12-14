use super::{encode_content_type, encode_uintvar, encode_wap_headers};
use crate::types::{message_header::MessageHeader, multipart::MultiPartItem};

use mime::Mime;

pub trait EncodableBody {
    fn content_type(&self) -> &mime::Mime;
    fn encode(self) -> Vec<u8>;
}

pub trait Item: Into<MultiPartItem> {
    // type SharedState;

    fn multipart_type(&self) -> mime::Mime;
    fn fianilize_encoder(_: &mut EncoderBuilder<Self>) {}
    // fn initalize_shared_state() -> Self::SharedState;
}

pub trait LayoutItem: Item {
    fn generate_layout(enc: &mut EncoderBuilder<Self>);
}

impl EncodableBody for (mime::Mime, Vec<u8>) {
    fn content_type(&self) -> &mime::Mime {
        &self.0
    }

    fn encode(self) -> Vec<u8> {
        self.1
    }
}

pub struct EncoderBuilder<I: Item> {
    parts: Vec<I>,
    layout: Option<I>,
    // item_state: I::SharedState,
}

impl<I: Item> EncoderBuilder<I> {
    /// Create a new empty builder
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            layout: None,
            // item_state: I::initalize_shared_state(),
        }
    }

    /// Replace the currently present parts, if any, with the provided ones
    pub fn parts(&mut self, parts: Vec<I>) {
        let mut parts = parts;
        self.parts.append(&mut parts);
    }

    /// Add a part to the list of parts
    pub fn part(&mut self, part: I) {
        self.parts.push(part);
    }

    // TODO: Use Result instead of Option
    /// Finalize builder into a type that can be encoded
    pub fn build(mut self) -> Option<MultiPartEncoder> {
        I::fianilize_encoder(&mut self);

        if let Some(part) = self.layout {
            self.parts.insert(0, part);
        }

        let content_type = self.parts.first().unwrap().multipart_type();

        Some(MultiPartEncoder {
            parts: self.parts.drain(..).map(|i| i.into()).collect(),
            content_type,
        })
    }
}

pub struct MixedItem {
    item: MultiPartItem,
}

impl MixedItem {
    pub fn new(item: MultiPartItem) -> Self {
        Self { item }
    }
}

impl Into<MultiPartItem> for MixedItem {
    fn into(self) -> MultiPartItem {
        self.item
    }
}

impl Item for MixedItem {
    fn multipart_type(&self) -> mime::Mime {
        "application/vnd.wap.multipart.mixed".parse().unwrap()
    }
}

pub struct RelatedBodyPart {
    item: MultiPartItem,
    id: String,
}

impl RelatedBodyPart {
    pub fn new(
        content_type: Mime,
        body: Vec<u8>,
        id: String,
        location: String,
    ) -> Self {
        let item = MultiPartItem {
            content_type,
            headers: vec![
                MessageHeader::ContentId(id.clone()),
                MessageHeader::ContentLocation(location.clone()),
            ],
            body,
        };

        Self { item, id }
    }
}

impl Into<MultiPartItem> for RelatedBodyPart {
    fn into(self) -> MultiPartItem {
        self.item
    }
}

impl Item for RelatedBodyPart {
    fn multipart_type(&self) -> mime::Mime {
        format!(
            "application/vnd.wap.multipart.related; start=\"{}\"; type=\"{}\"",
            self.id,
            self.item.content_type.essence_str()
        )
        .parse()
        .unwrap()
    }

    fn fianilize_encoder(e: &mut EncoderBuilder<RelatedBodyPart>) {
        // TODO: This check should be handled by the EncoderBuilder
        if e.layout.is_some() {
            // Ther's no return type here so rust's usual patterns for early returns don't fit well
            return;
        }

        let blank_layout_part = RelatedBodyPart::new(
            "application/smil".to_string().parse().unwrap(),
            "<smil><head><layout><root-layout width=\"818px\" height=\"480px\"/></layout></head><body><par dur=\"5000ms\"/></body></smil>".as_bytes().to_vec(),
            "<smil>".to_string(),
            "smil.xml".to_string(),
        );

        e.layout = Some(blank_layout_part);
    }
}

pub struct MultiPartEncoder {
    parts: Vec<MultiPartItem>,
    content_type: Mime,
}

impl EncodableBody for MultiPartEncoder {
    fn content_type(&self) -> &Mime {
        &self.content_type
    }

    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.append(&mut encode_uintvar(self.parts.len() as u64));
        for mut part in self.parts {
            let mut headers = encode_wap_headers(part.headers);
            let mut content_type = encode_content_type(part.content_type);

            buf.append(&mut encode_uintvar(
                (headers.len() + content_type.len()) as u64,
            ));
            buf.append(&mut encode_uintvar(part.body.len() as u64));
            buf.append(&mut content_type);
            buf.append(&mut headers);
            buf.append(&mut part.body);
        }

        buf
    }
}
