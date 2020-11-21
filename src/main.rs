use mms_parser::{parse_mms_pdu, parse_wap_push};
use std::{
    fs::File,
    io::{prelude::*, Read},
    path::PathBuf,
};
use structopt::StructOpt;

use isahc::{prelude::*, HttpClient};

#[derive(StructOpt, Debug)]
#[structopt(name = "mmsutil")]
struct AppArgs {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    Fetch(FetchArgs),
    Cat(CatArgs),
}

#[derive(StructOpt, Debug)]
struct CatArgs {
    /// Notification file to display
    #[structopt(name = "File", parse(from_os_str))]
    file: PathBuf,
}

#[derive(StructOpt, Debug)]
struct FetchArgs {
    #[structopt(flatten)]
    netargs: NetArgs,
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
    /// Save the response from the server to a file, very useful for debugging
    #[structopt(name = "Response", parse(from_os_str))]
    response: Option<PathBuf>
}

#[derive(StructOpt, Debug)]
struct NetArgs {
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
}

fn main() {
    let args = AppArgs::from_args();

    match args.cmd {
        Command::Fetch(args) => fetch(args),
        Command::Cat(args) => cat(args),
    }
}

fn cat(args: CatArgs) {
    let data = read_file(&args.file).expect("Could not read data file");

    // X-Mms-Message-Type must always be the first header of any mms pdu we can
    // use this to tell wether the provided data is a mms pdu, or a wap pdu
    // the binay value for X-Mms-Message-Type is 0x0C
    match data[0] {
        0x8C => {
            let (_remainder, mut parsed) =
                parse_mms_pdu(&*data).expect("Unable to parse provided data file");
            parsed.body = vec![];
            println!("{:#?}", parsed);
        }
        _ => {
            let (_, parsed) = parse_wap_push(&data).unwrap();
            println!("{:#?}", parsed)
        }
    }
}

fn fetch(args: FetchArgs) {
    if args.output.exists() {
        panic!("Please provide an output file which doesn't exist");
    }

    if let Some(ref resp) = args.response {
        if resp.exists() {
            panic!("Please provide a response file which doesn't exist");
        }
    }
    let data = read_file(&args.file).unwrap();

    let (_, parsed) = parse_wap_push(&data).unwrap();
    let body = parsed.parse_body().unwrap();

    let message_url = body.x_mms_content_location().unwrap();

    let mut client = HttpClient::builder().redirect_policy(isahc::config::RedirectPolicy::Follow);

    if let Some(interface) = args.netargs.interface {
        client = client.interface(isahc::config::NetworkInterface::name(interface));
    }

    let proto = if args.netargs.ipv6 == args.netargs.ipv4 {
        if args.netargs.ipv6 {
            println!("Warning: using the ipv6, and ipv4 flags together does nothing")
        }
        isahc::config::IpVersion::Any
    } else if args.netargs.ipv6 {
        isahc::config::IpVersion::V6
    } else {
        isahc::config::IpVersion::V4
    };

    client = client.ip_version(proto);

    let client = client.build().unwrap();

    let response: Vec<u8> = {
        let mut responce = client.get(message_url).unwrap();
        if !responce.status().is_success() {
            panic!(
                "Recieved error while trying to fetch message: {:#?}",
                responce
            );
        }
        let mut buffer = Vec::new();
        responce.body_mut().read_to_end(&mut buffer).unwrap();
        buffer
    };

    let body = match parse_mms_pdu(&*response) {
        Ok((_, mut parsed )) => {
            let body = parsed.body;
            parsed.body = vec![];
            println!("Message Response Headers: {:#?}", parsed);

            if let Some(response_location) = args.response {
                // TODO: We should probably continue and print instead of failing and printing an
                // error
                write_file(&response_location, &*response).expect("Unable to save the response from the server");
            }

            body
        },
        Err(err) => {
            println!("{:?}", err);
            println!("WARNING: could not parse response from server, saving response anyway");
            response
        }
    };

    write_file(&args.output, &*body).unwrap();
}

fn write_file(path: &PathBuf, data: &[u8]) -> std::io::Result<()> {
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
