use mms_parser::parse_data;
use std::{
    fs::File,
    io::{prelude::*, Read},
    path::PathBuf,
};
use structopt::StructOpt;

use isahc::{prelude::*, HttpClient};

#[derive(StructOpt, Debug)]
#[structopt(name = "mmsutil")]
struct Args {
    /// Use ipv6 only, sometimes carriers will only allow fetching messages using ipv6
    #[structopt(short = "6", long)]
    ipv6: bool,
    /// Use ipv4 only
    #[structopt(short = "4", long)]
    ipv4: bool,
    /// Dns servers to use, sometimes it's necessary to specifically use your carrier's dns servers
    #[structopt(short, long)]
    dns: Option<String>,
    /// Network interface to fetch mms messages on
    #[structopt(short, long)]
    interface: Option<String>,
    /// A file containing the mms notification.
    ///
    /// This will usually be created using `mmcli -s <Message ID>
    /// --create-file-with-data=<Notification>` see `man mmcli` or `mmcli --help` for more
    /// information
    #[structopt(name = "Notification", parse(from_os_str))]
    file: PathBuf,
    /// The file to store the downloaded mms message in
    ///
    /// Note: This file must not exist
    #[structopt(name = "Output", parse(from_os_str))]
    output: PathBuf,
}

fn main() {
    let args = Args::from_args();
    let data = read_file(&args.file).unwrap();

    let (_, parsed) = parse_data(&data).unwrap();
    let body = parsed.parse_body().unwrap();

    let message_url = body.x_mms_content_location().unwrap();

    let mut client = HttpClient::builder().redirect_policy(isahc::config::RedirectPolicy::Follow);

    if let Some(interface) = args.interface {
        client = client.interface(isahc::config::NetworkInterface::name(interface));
    }

    let proto = if args.ipv6 == args.ipv4 {
        if args.ipv6 {
            println!("Warning: using the ipv6, and ipv4 flags together does nothing")
        }
        isahc::config::IpProtocol::Any
    } else if args.ipv6 {
        isahc::config::IpProtocol::V6
    } else {
        isahc::config::IpProtocol::V4
    };

    client = client.ip_protocol(proto);

    let client = client.build().unwrap();

    let body: Vec<u8> = {
        let mut responce = client.get(message_url).unwrap();
        if !responce.status().is_success() {
            panic!("Recieved error while trying to fetch message: {:#?}", responce);
        }
        let mut buffer = Vec::new();
        responce.body_mut().read_to_end(&mut buffer).unwrap();
        buffer
    };

    write_file(&args.output, &*body).unwrap();
}

fn write_file(path: &PathBuf, data: &[u8]) -> std::io::Result<()> {
    if path.exists() {
        panic!("Output file path must not exist");
    }
    let mut file = File::create(path)?;

    file.write_all(data)?;

    Ok(())
}

fn read_file(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
