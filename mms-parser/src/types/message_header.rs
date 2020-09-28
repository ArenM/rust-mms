#[derive(Debug)]
#[non_exhaustive]
/// This only includes the fields needed to parse mms push notifications
/// this should be changed in the future
// TODO: this might be overkill, it might be easier to just translate
// header bytes to string names, and create a hashmap of them
pub enum MessageHeader {
    XWapApplicationId(usize),
    // PushFlag,
    // EncodingVersion,
    ContentLength(usize),
    // XWapInitiatorUri,
    AcceptCharset(u8), // For now
    // AcceptRanges,
    UnknownHeader(u8),
}
