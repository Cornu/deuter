use std::io;
use std::fmt;
use std::error::Error as StdError;

use self::Error::{Io, Connection, Stream};
use self::ConnectionError::{Protocol, FrameSize};
use self::StreamError::{Closed};

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Connection(ConnectionError),
    Stream(StreamError),
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
            Connection(ref e) => e.description(),
            Stream(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Io(ref e) => Some(e),
            Connection(ref e) => Some(e),
            Stream(ref e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ConnectionError> for Error {
    fn from(err: ConnectionError) -> Error {
        Error::Connection(err)
    }
}

impl From<StreamError> for Error {
    fn from(err: StreamError) -> Error {
        Error::Stream(err)
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    Protocol,
    FrameSize,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for ConnectionError {
    fn description(&self) -> &str {
        match *self {
            Protocol => "detected an unspecific protocol error",
            FrameSize => "received a frame with an invalid size",
        }
    }
}

#[derive(Debug)]
pub enum StreamError {
    Closed,
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for StreamError {
    fn description(&self) -> &str {
        match *self {
            Closed => "received frame after stream was half-closed",
        }
    }
}
