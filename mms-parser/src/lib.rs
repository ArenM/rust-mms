mod types;

use types::read_uintvar;
use types:: {Wap, PduType, PushMessageBody};

use nom::number::complete::be_u8;
use nom::IResult;

pub fn parse_data(data: &[u8]) -> IResult<&[u8], Wap> {
    // TODO: This should ONLY be red in "connectionless PDUs"
    let (r, tid) = be_u8(data)?;

    // TODO: Convert this to an enum based on table 34 in assigned numbers from
    // wap-230-wsp-20010705-a.pdf
    let (r, message_type) = be_u8(r)?;
    let (r, body) = parse_message_body(r, PduType::from(message_type))?;

    Ok((
        r,
        Wap {
            transaction_id: tid,
            message_type: PduType::from(message_type),
            body
        },
    ))
}

// TODO: PushMessageBody is all I care about for now, but this should be fixed
fn parse_message_body(d: &[u8], _message_type: PduType) -> IResult<&[u8], PushMessageBody> {
    let (d, header_length) = read_uintvar(d)?;
    Ok((d, PushMessageBody { header_length}))
}
