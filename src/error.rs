use std::io;
use std::fmt;
use std::error::Error as StdError;

use self::Error::{Io, Connection, Protocol, FrameSize};

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Connection,
    Protocol,
    FrameSize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Io(ref e) => e.description(),
            Connection => "Connection Error",
            Protocol => "Protocol Error",
            FrameSize => "Frame Size Error",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Io(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

