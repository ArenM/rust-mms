mod helpers;
mod types;

use helpers::{null_delimited, tag_null, u8_to_string};
use types::read_uintvar;
use types::{PduType, PushMessageBody, Wap};

#[macro_use]
extern crate nom;

use nom::{do_parse, named, number::complete::be_u8};

named!(pub parse_data<Wap>,
    do_parse!(
        // TODO: This should ONLY be red in "connectionless PDUs"
        tid: be_u8 >>
        message_type: be_u8 >>
        body: parse_message_body >>
        (Wap {
                transaction_id: tid,
                message_type: PduType::from(message_type),
                body,
        })
    )
);

named!(pub parse_message_body<PushMessageBody>,
    do_parse!(
        header_length:  read_uintvar    >>
        content_type:   null_delimited  >>
        tag_null >>
        some_number:    read_uintvar    >>
        string:         null_delimited  >>
        (PushMessageBody {
            header_length,
            content_type: u8_to_string(content_type),
            random_number: some_number,
            string: u8_to_string(string),
        })
    )
);
