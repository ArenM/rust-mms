This project aims to eventually have equivalent functionality to mmsd. instead
of relying on ofono it will support using ModemManager to talk to the modem.
ideally it will use modular approach, so as to support both ModemManager and
ofono.

# Roadmap
- [x] Parse mms push notifications
- [x] Fetch messages when passed a binary message, for example from ModemManager
  with `mmcli -s n --create-file-with-data="mms1.bin"`
  - [ ] Handle multi-part messages. Often a message will contain multiple files
   in the body field, for example a
   [smil](https://en.wikipedia.org/wiki/synchronized_multimedia_integration_language)
   section, and an image.
- [ ] Encode messages + cli to send messages (the user must use mmcli to send
  any data which needs to be encoded as sms messages)
- [ ] Integrate with ModemManager through a generic interface (wrapper around
  ModemManager DBus interface) to interact with messages without manually running
  mmcli.
- [ ] Service - run in the background and download messages as they arrive
- [ ] DBus interface - have a dbus interface for listening for new messages,
  retrieving message data, sending messages... etc. This will be necessary to
  integrate into messaging apps.

# Building
Dependencies:
On postmarketos I had to install `openssl-dev` to get it to build. I'm not sure
about others.

`cargo build` should work, `cargo build --release` takes longer, but will
produce a smaller binary which should run faster.

# Optional -- Installation
copy `target/release/mmsutil` or `target/debug/mmsutil` to a directory in
your `$path`, ie. `/usr/local/bin/`.  

# Running
use `cargo run -- --help` or `rust-mmsd --help` if you installed using the
previous step to get usage information.

To send or receive a message you must connect to the MMSC through the mode, which is the
typically wwan0 interface on the PinePhone. It is sometimes also necessary to
perform DNS lookups using your carriers DNS servers.

The MMSC is your carriers server that handle MMS messages. The easiest method to
find it is by searching for apn settings for your carrier, and looking for the
MMSC entry.

## Fetching Messages
Use `mmcli -s <Message ID> --create-file-with-data=<Notification>` to save the
push notification to a file that can be passed to `mmsutil`.

Run `mmsutil fetch <Notification> <Output File>` to download the message. You
may also need to use the `--dns` and `--interface` flags to download messages
through the MMSC. See `mmsutil fetch --help` for more information.

## Sending Messages
Sending messages is still a work in progress however if you can get any useful
errors I'd like to know.

Messages can be encoded using `mmutil encode` see `mmutil encode --help` for
more information about encoding messages.

Once you have an encoded message it can be sent using something like `curl -vv
--interface wwan0 --data-binary "@encoded-message.bin" -H "Content-Type:
application/vnd.wap.mms-message" -H "Expect:"
"${mmsc}" -o response.bin`
