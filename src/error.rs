use std::io;
use std::fmt;
use std::error;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<error::Error>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ErrorKind {
    // The associated condition is not a result of an error.
    // No = 0x0,
    /// The endpoint detected an unspecific protocol error.
    Protocol = 0x1,
    /// The endpoint encountered an unexpected internal error.
    Internal = 0x2,
    /// The endpoint detected that its peer violated the flow-control protocol.
    FlowControl = 0x3,
    // The endpoint sent a SETTINGS frame but did not receive a response in a timely manner.
    // SettingsTimeout = 0x4,
    // The endpoint received a frame after a stream was half-closed.
    // StreamClosed = 0x5,
    /// The endpoint received a frame with an invalid size.
    FrameSize = 0x6,
    /// The endpoint refused the stream prior to performing any application processing.
    RefusedStream = 0x7,
    /// Used by the endpoint to indicate that the stream is no longer needed.
    Cancel = 0x8,
    /// The endpoint is unable to maintain the header compression context for the connection.
    Compression = 0x9,
    /// The connection established in response to a CONNECT request was reset or abnormally closed.
    Connect = 0xa,
    /// The endpoint detected that its peer is exhibiting a behavior
    /// that might be generating excessive load.
    EnhanceYourCalm = 0xb,
    /// The underlying transport has properties that do not meet minimum security requirements.
    InadequateSecurity = 0xc,
    /// The endpoint requires that HTTP/1.1 be used instead of HTTP/2.
    Http11Required = 0xd,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
        where E: Into<Box<error::Error>>
    {
        Error {
            kind: kind,
            error: error.into(),
        }
    }

    pub fn protocol<E>(error: E) -> Error
        where E: Into<Box<error::Error>>
    {
        Self::new(ErrorKind::Protocol, error)
    }

    pub fn frame_size<E>(error: E) -> Error
        where E: Into<Box<error::Error>>
    {
        Self::new(ErrorKind::FrameSize, error)
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(fmt)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&error::Error> {
        self.error.cause()
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error {
            kind: ErrorKind::Internal,
            error: Box::new(err),
        }
    }
}
