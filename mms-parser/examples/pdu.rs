use mms_parser::{split_header_fields, parse_header_fields, parse_header_fields_with_errors, MessageClass, ParserCtx};
use mms_parser::types::VndWapMmsMessage;
use mms_parser::parse_wap_push;

use std::{fs::File, io::Read, path::PathBuf};
 
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = pico_args::Arguments::from_env();
    let path: String = args.value_from_str("--file")?;
    let has_body: bool = args.contains("-b") | args.contains("--body");
    let is_wap: bool = args.contains("-w") | args.contains("--wap");
    let include_errors: bool = args.contains("-e") | args.contains("--errors");

    let data = read_file(&path.into()).unwrap();

    let ctx = ParserCtx {
        message_class: MessageClass { has_body },
    };

    let body_data = if is_wap {
        let (_, wap) = nom::dbg_dmp(parse_wap_push, "something")(&data).unwrap();
        // println!("Notification Headers: {:#?}", wap);
        wap.data
    } else {
        data
    };

    let (_, split) = split_header_fields(&*body_data, ctx).unwrap();

    match include_errors {
        true => {
            let parsed = parse_header_fields_with_errors(&split);
            println!("Notification Body: {:#?}", parsed);
        }
        false => {
            let parsed = parse_header_fields(&split);
            println!("Notification Body: {:#?}", VndWapMmsMessage::new(parsed));
        }
    }

    Ok(())
}

fn read_file(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
