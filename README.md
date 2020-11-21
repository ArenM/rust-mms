This project aims to eventually have equivalent functionality to mmsd. instead
of relying on ofono it will support using ModemManager to talk to the modem.
ideally it will use modular approach, so as to support both ModemManager and
ofono.

# roadmap
- [x] parse mms push notifications
- [x] fetch messages when passed a binary message, ie. from `mmcli -s n
  --create-file-with-data="mms1.dat"`
 - [ ] handle multi-part messages. often a message will contain multiple files
   in the body field, for example a
   [smil](https://en.wikipedia.org/wiki/synchronized_multimedia_integration_language)
   section, and an image.
- [ ] encode messages + cli to send messages (the user must use mmcli to send
  any data which needs to be encoded as sms messages)
- [ ] integrate with modemmanager through a generic interface (wrapper around
  modemmanager dbus interface) to interact with messages without manually running
  mmcli.
- [ ] service - run in the background and download messages as they arrive
- [ ] dbus interface - have a dbus interface for listening for new messages,
  retrieving message data, sending messages... etc. this will be necessary to
  integrate into messaging apps.

# building
`cargo build` should work, `cargo build --release` takes longer, but will
produce a smaller binary which should run faster. On postmarketos I had to
install `openssl-dev` to get it to build, I'm not sure about other dependencies. 

# optional -- installation
copy `target/release/mmsutil` or `target/debug/mmsutil` to a directory in
your `$path`, ie. `/usr/local/bin/`.  

# running
use `cargo run -- --help` or `rust-mmsd --help` if you installed using the
previous step to get usage information.

The notification parameter
