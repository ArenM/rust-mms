use mms_parser::parse_data;
use clap::{App, Arg};

use std::{fs::File, io::Read, path::PathBuf};

fn main() {
    let matches = App::new("MMS Notification Parser")
        .arg(
            Arg::with_name("file")
                .help("Binary file containing mms notification")
                .required(true)
                .value_name("FILE")
                .index(1)
                .takes_value(true),
        )
        .get_matches();

    let path = matches.value_of("file").unwrap();
    let data = read_file(&path.into()).unwrap();

    let parsed = parse_data(&data);
    println!("Parsed: {:?}", parsed)
}

fn read_file(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
