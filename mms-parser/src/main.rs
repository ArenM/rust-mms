use mms_parser::parse_data;

use std::{fs::File, io::Read, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: add command line / environment variable options
    // for with_level, and with_module_level
    simple_logger::SimpleLogger::new().init().unwrap();

    let mut args = pico_args::Arguments::from_env();
    let path: String = args.value_from_str("--file")?;

    let data = read_file(&path.into()).unwrap();

    let parsed = parse_data(&data).unwrap();
    println!("Parsed: {:?}", parsed);
    Ok(())
}

fn read_file(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
