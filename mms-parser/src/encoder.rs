pub(crate) mod mms_header;

fn encode_string(v: String) -> Vec<u8> {
    let mut bytes: Vec<u8> = v.into_bytes().to_vec();
    bytes.push(0);
    if (32..=127).contains(&bytes[0]) {
        bytes.insert(0, '"' as u8);
    }
    bytes
}
