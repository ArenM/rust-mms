use mms_parser::{
    encoder::encode_mms_message,
    types::{
        mms_header::{MessageTypeField, MmsHeader, MmsHeaderValue},
        VndWapMmsMessage,
    },
};

use ordered_multimap::ListOrderedMultimap as MultiMap;
use promptly::prompt;

use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

const MIME_ERROR_MESSAGE: &str = "Couldn't determine content type from provided file";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body_path: PathBuf = prompt("File to send")?;
    if !body_path.is_file() {
        panic!("File must exist")
    }

    let extension: &str = &body_path
        .extension()
        .expect(MIME_ERROR_MESSAGE)
        .to_str()
        .expect(MIME_ERROR_MESSAGE);

    let mime_type: mime::Mime = mime_db::lookup(extension)
        .expect(MIME_ERROR_MESSAGE)
        .parse()
        .expect(MIME_ERROR_MESSAGE);
    let save: PathBuf = prompt("File to output encoded message to")?;

    if save.exists() {
        panic!("The output file you specified already exists")
    }

    let to: u64 = prompt("Number to send the message to")?;
    let from: u64 = prompt("Number to report sending the message from")?;

    let mut message = VndWapMmsMessage::new(MultiMap::new());

    message.headers.insert(
        MmsHeader::XMmsMessageType,
        MmsHeaderValue::MessageTypeField(MessageTypeField::MSendReq),
    );

    message
        .headers
        .insert(MmsHeader::XMmsTransactionId, "id".to_string().into());

    message
        .headers
        .insert(MmsHeader::XMmsMMSVersion, mms_parser::MMS_VERSION.into());

    message
        .headers
        .insert(MmsHeader::To, format!("+{}/TYPE=PLMN", to).into());

    message
        .headers
        .insert(MmsHeader::From, format!("+{}/TYPE=PLMN", from).into());

    let mut file = File::open(body_path)?;
    file.read_to_end(&mut message.body)?;
    // println!("Generated Messag: {:#?}", message);

    let encoded = encode_mms_message(message.headers, (mime_type, message.body));

    let mut out = File::create(save)?;
    out.write_all(&encoded)?;

    Ok(())
}
