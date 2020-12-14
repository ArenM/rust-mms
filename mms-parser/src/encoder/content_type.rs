use super::*;
use crate::types::content_type_codes::CONTENT_TYPE_CODES;

enum ContentEssence {
    Short(u8),
    Long(String),
}

fn short_content_type(essence_str: &str) -> ContentEssence {
    CONTENT_TYPE_CODES
        .iter()
        .find(|(_, long)| long == &essence_str)
        .map(|(id, _)| ContentEssence::Short(id.clone()))
        .unwrap_or(ContentEssence::Long(essence_str.to_owned()))
}

fn param(name: mime::Name, value: mime::Name) -> Vec<u8> {
    use mime::*;
    // From wap-230-wsp table 38
    match &*name.as_str().to_lowercase() {
        "charset" => {
            let mut buf = vec![0x01];

            if value == STAR {
                buf.push(128);
            } else {
                // Encoders seem to need to be able to return errors
                well_known_charset(value.as_str()).unwrap();
            }

            buf
        }
        "name" => {
            let mut buf = encode_short_integer(0x05).unwrap();
            buf.append(&mut super::encode_string(value.as_str().to_owned()));
            buf
        }
        "type" => {
            let mut buf = encode_short_integer(0x09).unwrap();
            buf.append(&mut constrained_encoding(value.as_str()));
            buf
        }
        "start" => {
            let mut buf = encode_short_integer(0x0A).unwrap();
            buf.append(&mut encode_string(value.as_str().to_owned()));

            buf
        }
        _ => untyped_param(name, value),
    }
}

fn params(params: mime::Params) -> Vec<u8> {
    let enc: Vec<Vec<u8>> =
        params.map(|(name, value)| param(name, value)).collect();
    enc.concat()
}

fn well_known_charset(chr_set: &str) -> Option<u8> {
    // From http://www.iana.org/assignments/character-sets/character-sets.xhtml
    Some(match chr_set {
        "US-ASCII" => 3,
        "ISO-8859-1" => 4,
        "UTF-8" => 106,
        _ => return None,
    })
}

fn untyped_param(_name: mime::Name, _value: mime::Name) -> Vec<u8> {
    unimplemented!()
}

fn general_form(essence: &str, mut params: Vec<u8>) -> Vec<u8> {
    use ContentEssence::*;
    let mut buf = vec![];

    let mut ct = match short_content_type(essence) {
        // I don't think this needs to be longer than a short integer, but if it
        // does, the spec says this can be changed to a integer-value which is
        // either a short or a long integer
        Short(c) => encode_short_integer(c).unwrap(),
        Long(c) => encode_string(c),
    };

    buf.append(&mut ct);
    buf.append(&mut params);

    value_length(buf)
}

fn constrained_encoding(essence: &str) -> Vec<u8> {
    let essence = short_content_type(essence);

    match essence {
        ContentEssence::Short(c) => encode_short_integer(c).unwrap(),
        ContentEssence::Long(c) => encode_string(c.to_owned()),
    }
}

pub(crate) fn encode_content_type(content_type: mime::Mime) -> Vec<u8> {
    // let essence = short_content_type(content_type.essence_str());
    let params = params(content_type.params());

    if params.len() > 0 {
        general_form(content_type.essence_str(), params)
    } else {
        constrained_encoding(content_type.essence_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn short_well_known() {
        let encoded = encode_content_type(mime::TEXT_PLAIN);
        assert_eq!(encoded, vec![0x03 | 0x80]);
    }

    #[test]
    fn simple_string() {
        let encoded = encode_content_type("unknown/type".parse().unwrap());
        assert_eq!(encoded, "unknown/type\x00".as_bytes());
    }

    #[test]
    fn well_known_charset_well_known_param() {
        let encoded = encode_content_type(
            "application/vnd.wap.multipart.related; start=\"<text>\"; type=\"text/plain\""
            .parse()
            .unwrap());

        assert_eq!(encoded, b"\x0B\xB3\x8A<text>\0\x89\x83");
    }

    #[test]
    #[ignore]
    // TODO: make this test pass
    fn well_known_charset_unknown_param() {
        let encoded = encode_content_type(
            "application/vnd.wap.multipart.related; unknown=\"unknown\""
                .parse()
                .unwrap(),
        );

        assert_eq!(encoded, b"\xB3unknown\0unknown\0\x09\x83");
    }
}
