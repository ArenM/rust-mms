// use nom::take_till1;
use nom::{do_parse, named};

pub fn u8_to_string(i: &[u8]) -> String {
    String::from_utf8(i.to_vec()).unwrap()
}

named!(pub null_delimited<&[u8]>,
    do_parse!(
        items: take_till1!(|c| c == 0u8) >>
        tag!(&[0u8]) >>
        (items)
    )
);
