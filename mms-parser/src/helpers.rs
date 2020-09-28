use nom::{do_parse, named};
// use encoding::{Encoding, DecoderTrap};
// use encoding::all::UTF_8;

pub fn u8_to_string(i: &[u8]) -> String {
    String::from_utf8(i.to_vec()).unwrap()
}

// pub fn u8_to_string_include_non_utf8(d: &[u8]) -> String {
//     UTF_8.decode(d, DecoderTrap::Replace).unwrap()
// }

named!(pub null_delimited<&[u8]>,
    do_parse!(
        items: take_till1!(|c| c == 0u8) >>
        tag!(&[0u8]) >>
        (items)
    )
);
