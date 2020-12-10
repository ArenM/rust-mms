use mms_parser::{
    encoder::encode_mms_message,
    parse_mms_pdu, parse_wap_push,
    types::{
        message_header::MessageHeader,
        mms_header::{FromField, MessageTypeField, MmsHeader, MmsHeaderValue},
        VndWapMmsMessage,
    },
};

use std::{
    fs::{DirBuilder, File},
    io::{prelude::*, Read},
    path::PathBuf,
};

#[macro_use]
extern crate anyhow;
use anyhow::{Context, Result};

use isahc::{prelude::*, HttpClient};
use structopt::StructOpt;
use uuid::Uuid;

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
    Decode(DecodeArgs),
    Encode(EncodeArgs),
}

#[derive(StructOpt, Debug)]
struct EncodeArgs {
    /// Your phone number
    #[structopt(short, long)]
    from: Option<u64>,
    /// The number of the recipient of this message
    #[structopt(
        short,
        long,
        required_unless("unchecked-to"),
        conflicts_with("unchecked-to")
    )]
    to: Option<u64>,
    /// Used to send a message to something other than a mobile phone number
    #[structopt(long)]
    unchecked_to: Option<String>,
    /// Subject of the message
    #[structopt(long)]
    subject: Option<String>,
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
struct DecodeArgs {
    /// MMS Message to decode
    #[structopt(name = "File", parse(from_os_str))]
    file: PathBuf,
    /// Directory to save message data in
    ///
    /// A subdirectory with the id of the message will be created
    #[structopt(name = "Output", parse(from_os_str))]
    out: PathBuf,
}

#[derive(StructOpt, Debug)]
struct FetchArgs {
    #[structopt(flatten)]
    netargs: NetArgs,
    /// A file containing the mms notification.
    ///
    /// This will usually be created using `mmcli -s <Message ID>
    /// --create-file-with-data=<Notification>` see `man mmcli` or `mmcli
    /// --help` for more information
    #[structopt(name = "Notification", parse(from_os_str))]
    file: PathBuf,
    /// The directory to store the downloaded data in
    ///
    /// A subdirectory with the id of the message will be created
    #[structopt(name = "Output", parse(from_os_str))]
    output: PathBuf,
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

fn main() -> anyhow::Result<()> {
    let args = AppArgs::from_args();

    match args.cmd {
        Command::Fetch(args) => fetch(args)?,
        Command::Cat(args) => cat(args),
        Command::Decode(args) => command_decode(args)?,
        Command::Encode(args) => encode_to_file(args),
    }

    Ok(())
}

fn cat(args: CatArgs) {
    pager::Pager::with_default_pager("less").setup();
    let data = read_file(&args.file).expect("Could not read data file");

    // X-Mms-Message-Type must always be the first header of any mms pdu we can
    // use this to tell weather the provided data is a mms pdu, or a wap pdu
    // the binary value for X-Mms-Message-Type is 0x0C
    match data[0] {
        0x8C => {
            println!("Mms Data");

            let (_remainder, parsed) = parse_mms_pdu(&*data)
                .expect("Unable to parse provided data file");

            println!("Headers: {:#?}", parsed.headers);

            if parsed.body.len() > 0 {
                if parsed.has_multipart_body() {
                    let body = mms_parser::parse_multipart_body(&parsed.body)
                        .unwrap()
                        .1;
                    println!("Body: {:#?}", body);
                } else {
                    let body = String::from_utf8_lossy(&parsed.body);
                    println!("Body: {}", body);
                }
            }
        }
        _ => {
            println!("Type: WAP Data");

            let (_, parsed) = parse_wap_push(&data).unwrap();
            println!("Wap Push Headers: {:#?}", parsed);

            let body =
                parsed.parse_body().expect("Unable to parse wap push body");
            println!("Wap Push Body: {:#?}", body);
        }
    }
}

fn command_decode(args: DecodeArgs) -> anyhow::Result<()> {
    let data = read_file(&args.file).expect("Could not read data file");

    if data[0] != 0x8C {
        bail!("Unknown data type, please provide a mms pdu");
    }

    let (_remainder, message) =
        parse_mms_pdu(&*data).expect("Unable to parse provided data file");

    println!("Headers: {:#?}", message.headers);

    if message.body.len() == 0 {
        println!(
            "WARNING: data file contained no body part, no new data was saved"
        );
        return Ok(());
    }

    let mut out = args.out.clone();
    out.push(message.message_id().unwrap_or(&Uuid::new_v4().to_string()));

    // TODO: check for file conflicts instead of message id conflicts
    if out.exists() {
        bail!("It looks like a message with the same id has already \
                been decoded, if you would like to overwrite it can remove {:?}", out);
    }

    DirBuilder::new().create(&out)?;

    save_body(&message, out)
}

fn save_body(
    message: &mms_parser::types::VndWapMmsMessage,
    mut out: PathBuf,
) -> anyhow::Result<()> {
    use MessageHeader::ContentLocation;
    if message.has_multipart_body() {
        let body = mms_parser::parse_multipart_body(&message.body)
            .map_err(|e| {
                // TODO: This is the correct solution here, however it's blocked by
                // https://github.com/Geal/nom/issues/1254
                // e.to_owned();
                anyhow!(e.to_string())
            })?
            .1;
        let mut error = Ok(());

        for item in body {
            let content_location = item
                .headers
                .iter()
                .find_map(|h| {
                    if let ContentLocation(h) = h {
                        Some(h.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Uuid::new_v4().to_string());

            let mut file_path = out.clone();
            file_path.push(content_location);

            error = error.or(write_file(&file_path, &*item.body))
        }

        error?;
    } else {
        let content_type = if let Some(c) = message.content_type() {
            c
        } else {
            // If there's no content-type then there's no body
            return Ok(());
        };

        out.push("Unnamed-1");
        out.set_extension(mime_db::extension(content_type).unwrap_or("bin"));
        write_file(&out, &*message.body)?;
    };

    Ok(())
}

fn encode_to_file(args: EncodeArgs) {
    const MIME_ERROR_MESSAGE: &str =
        "Couldn't determine content type from provided file";

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
        // this case would most likely be a bug in structopt
        panic!("Either args.to, or args.unchecked to must have a value");
    };

    message.headers.insert(
        MmsHeader::XMmsMessageType,
        MmsHeaderValue::MessageTypeField(MessageTypeField::MSendReq),
    );

    message.headers.insert(
        MmsHeader::XMmsTransactionId,
        Uuid::new_v4().to_string().into(),
    );

    message
        .headers
        .insert(MmsHeader::XMmsMMSVersion, mms_parser::MMS_VERSION.into());

    message
        .headers
        .insert(MmsHeader::XMmsDeliveryReport, true.into());

    message.headers.insert(MmsHeader::To, to.into());

    if let Some(from) = args.from {
        message.headers.insert(
            MmsHeader::From,
            FromField::Address(format!("+{}/TYPE=PLMN", from)).into(),
        );
    } else {
        message
            .headers
            .insert(MmsHeader::From, FromField::InsertAddress.into());
    }

    if let Some(subject) = args.subject {
        message.headers.insert(MmsHeader::Subject, subject.into());
    }

    message
        .headers
        .insert(MmsHeader::ContentType, mime_type.into());

    println!("Generated Message Headers: {:#?}", message.headers);

    let mut file = File::open(args.file).expect("Unable to open file to send");
    file.read_to_end(&mut message.body)
        .expect("Error reading file");

    let encoded = encode_mms_message(message);
    write_file(&args.output, &*encoded)
        .expect("Unable to save message to output");
}

fn fetch(args: FetchArgs) -> Result<()> {
    if !args.output.is_dir() {
        bail!("Output must be a directory that exists");
    }

    let data = read_file(&args.file).context("Notification File")?;

    let (_, parsed) = parse_wap_push(&data).unwrap();
    let body = parsed
        .parse_body()
        .ok_or(anyhow!("Failed to parse message notification"))?;

    let mut output = args.output.clone();
    output.push(body.x_mms_transaction_id().ok_or(
        anyhow!("Invalid message notification").context(
            "Message notifications must have a X-Mms-Transaction-ID field",
        ),
    )?);

    if output.exists() {
        bail!(
            "Files for a message with the same id already exist, if you would \
            like to overwrite them you can remove {:?}",
            output
        );
    }

    DirBuilder::new().create(&output)?;

    let message_url = body.x_mms_content_location().ok_or(
        anyhow!("Invalid MMs notification. Message notification must contain a content-location field.",
        ),
    )?;

    let mut client = HttpClient::builder()
        .redirect_policy(isahc::config::RedirectPolicy::Follow);

    if let Some(interface) = args.netargs.interface {
        client =
            client.interface(isahc::config::NetworkInterface::name(interface));
    }

    let proto = if args.netargs.ipv6 {
        isahc::config::IpVersion::V6
    } else if args.netargs.ipv4 {
        isahc::config::IpVersion::V4
    } else {
        isahc::config::IpVersion::Any
    };

    let client = client.ip_version(proto).build()?;

    let response: Vec<u8> = {
        let mut responce = client.get(message_url)?;

        let mut buffer = Vec::new();
        responce.body_mut().read_to_end(&mut buffer)?;

        let mut o = output.clone();
        o.push("m-retrieve-conf.bin");
        write_file(&o, &*buffer)
            .context("Could not save response from server")?;

        if !responce.status().is_success() {
            bail!(
                "Received error while trying to fetch message: {:#?}",
                responce
            );
        }

        buffer
    };

    // TODO: Depends on https://github.com/Geal/nom/issues/1254
    let (_remainder, parsed) = parse_mms_pdu(&*response).unwrap();
    // .context("Could not parse response from server")?;

    println!("Message Response Headers: {:#?}", parsed.headers);

    save_body(&parsed, output)?;
    Ok(())
}

fn write_file(path: &PathBuf, data: &[u8]) -> Result<()> {
    let mut file = File::create(path).with_context(|| {
        anyhow!("Could not create file to write: {:?}", path)
    })?;

    file.write_all(data)
        .with_context(|| anyhow!("Could not write data to file: {:?}", path))?;

    Ok(())
}

fn read_file(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
