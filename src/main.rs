use mms_parser::{
    encoder::encode_mms_message,
    parse_mms_pdu, parse_wap_push,
    types::{
        mms_header::{MessageTypeField, MmsHeader, MmsHeaderValue},
        VndWapMmsMessage,
    },
};

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
    Encode(EncodeArgs),
}

#[derive(StructOpt, Debug)]
struct EncodeArgs {
    /// Your phone number
    #[structopt(short, long)]
    from: u64,
    /// The number of the recipient of this message
    #[structopt(short, long, required_unless("unchecked-to"), conflicts_with("unchecked-to"))]
    to: Option<u64>,
    /// Used to send a message to something other than a mobile phone number
    #[structopt(long)]
    unchecked_to: Option<String>,
    /// File to send
    #[structopt(name = "File", parse(from_os_str))]
    file: PathBuf,
    /// File to save message to, must be sent using curl
    #[structopt(name = "Output", parse(from_os_str))]
    output: PathBuf,
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
    #[structopt(name = "Output", parse(from_os_str))]
    output: PathBuf,
    /// Save the response from the server to a file, very useful for debugging
    #[structopt(name = "Response", parse(from_os_str))]
    response: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct NetArgs {
    /// Use ipv6 only, sometimes carriers will only allow fetching messages using ipv6
    #[structopt(short = "6", long, group("ip_version"))]
    ipv6: bool,
    /// Use ipv4 only
    #[structopt(short = "4", long, group("ip_version"))]
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
        Command::Encode(args) => encode_to_file(args),
    }
}

fn cat(args: CatArgs) {
    pager::Pager::with_default_pager("less").setup();
    let data = read_file(&args.file).expect("Could not read data file");

    // X-Mms-Message-Type must always be the first header of any mms pdu we can
    // use this to tell wether the provided data is a mms pdu, or a wap pdu
    // the binay value for X-Mms-Message-Type is 0x0C
    match data[0] {
        0x8C => {
            let (_remainder, parsed) =
                parse_mms_pdu(&*data).expect("Unable to parse provided data file");
            println!("{:#?}", parsed);
        }
        _ => {
            let (_, parsed) = parse_wap_push(&data).unwrap();
            println!("Wap Push Headers: {:#?}", parsed);
            let body = parsed.parse_body().expect("Unable to parse wap push body");
            println!("Wap Push Body: {:#?}", body);
        }
    }
}

fn encode_to_file(args: EncodeArgs) {
    const MIME_ERROR_MESSAGE: &str = "Couldn't determine content type from provided file";

    if !args.file.is_file() {
        panic!("{:?}: file not found", args.file);
    }

    if args.output.exists() {
        panic!("Please provide an output file which doesn't exist");
    }

    let extension: &str = &args
        .file
        .extension()
        .expect(MIME_ERROR_MESSAGE)
        .to_str()
        .expect(MIME_ERROR_MESSAGE);

    let mime_type: mime::Mime = mime_db::lookup(extension)
        .expect(MIME_ERROR_MESSAGE)
        .parse()
        .expect(MIME_ERROR_MESSAGE);

    let mut message = VndWapMmsMessage::empty();

    let to = if let Some(to) = args.to {
        format!("+{}/TYPE=PLMN", to)
    } else if let Some(to) = args.unchecked_to {
        to
    } else {
        panic!("Either args.to, or args.unchecked to must have a value");
    };

    message.headers.insert(
        MmsHeader::XMmsMessageType,
        MmsHeaderValue::MessageTypeField(MessageTypeField::MSendReq),
    );

    message
        .headers
        .insert(MmsHeader::XMmsTransactionId, uuid::Uuid::new_v4().to_string().into());

    message
        .headers
        .insert(MmsHeader::XMmsMMSVersion, mms_parser::MMS_VERSION.into());

    message
        .headers
        .insert(MmsHeader::To, to.into());

    message
        .headers
        .insert(MmsHeader::From, format!("+{}/TYPE=PLMN", args.from).into());

    message
        .headers
        .insert(MmsHeader::ContentType, mime_type.into());

    println!("Generated Message Headers: {:#?}", message.headers);

    let mut file = File::open(args.file).expect("Unable to open file to send");
    file.read_to_end(&mut message.body)
        .expect("Error reading file");

    let encoded = encode_mms_message(message);
    write_file(&args.output, &*encoded).expect("Unable to save message to output");
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

    let proto = if args.netargs.ipv6 {
        isahc::config::IpVersion::V6
    } else if args.netargs.ipv4 {
        isahc::config::IpVersion::V4
    } else {
        isahc::config::IpVersion::Any
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
        Ok((_, mut parsed)) => {
            let body = parsed.body;
            parsed.body = vec![];
            println!("Message Response Headers: {:#?}", parsed);

            if let Some(response_location) = args.response {
                // TODO: We should probably continue and print instead of failing and printing an
                // error
                write_file(&response_location, &*response)
                    .expect("Unable to save the response from the server");
            }

            body
        }
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
