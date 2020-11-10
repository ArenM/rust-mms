use nom::{do_parse, named};

pub fn u8_to_string(i: &[u8]) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(i.to_vec())
}

named!(pub take_till_null<&[u8]>,
    do_parse!(
        items: take_till1!(|c| c == 0u8) >>
        tag!(&[0u8]) >>
        (items)
    )
);
