use nom::{do_parse, named};

pub fn u8_to_string(i: &[u8]) -> String {
    String::from_utf8(i.to_vec()).unwrap()
}

named!(pub tag_null<()>, do_parse!(tag!(&[0u8]) >> ()));

named!(pub null_delimited, take_till1!(|c| c == 0u8));
