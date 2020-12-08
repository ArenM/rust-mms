#[derive(Debug, Clone)]
#[non_exhaustive]
// TODO: this might be overkill, it might be easier to just translate
// header bytes to string names, and create a hashmap of them
pub enum MessageHeader {
    XWapApplicationId(usize),
    // PushFlag,
    // EncodingVersion,
    ContentLength(usize),
    // XWapInitiatorUri,
    AcceptCharset(u8), // This should change to a charset struct
    // AcceptRanges,
    ContentId(String),
    ContentLocation(String),
    UnknownHeader((u8, Vec<u8>)),
}
