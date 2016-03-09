pub mod settings;
pub mod headers;

use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use error::{Error, ErrorKind, Result};
use super::StreamId;
use self::settings::SettingsFrame;
use self::headers::HeadersFrame;

pub type Flags = u8;

trait Frame: Sized + Into<Vec<u8>> {
    fn size(&self) -> usize;
}

#[derive(Debug)]
pub enum FrameType {
    //Data,
    Headers(HeadersFrame),
    //Priority,
    //RstConn,
    Settings(SettingsFrame),
    //PushPromise,
    //Ping,
    //GoAway,
    //WindowUpdate,
    //Continuation,
    Unknown,
}

pub struct FrameHeader {
    payload_len: usize,
    frame_type: u8,
    flags: Flags,
    stream_id: StreamId,
}

impl FrameHeader {
    fn read<R: Read>(mut readr: R) -> Result<FrameHeader> {
        let mut buf = [0; 9];
        try!(readr.read_exact(&mut buf));
        Ok(FrameHeader {
            payload_len: BigEndian::read_uint(&mut buf, 3) as usize,
            frame_type: buf[3],
            flags: buf[4],
            stream_id: StreamId(BigEndian::read_u32(&mut buf[5..]) & 0x7FFFFFFF),
        })
    }
}

pub trait ReadFrame: Read + Sized {
    fn read_frame(&mut self, max_size: usize) -> Result<FrameType> {
        let header = try!(FrameHeader::read(self.by_ref()));
        if header.payload_len > max_size {
            return Err(Error::new(ErrorKind::FrameSize, "payload length exceeds max frame size setting"));
        }
        match header.frame_type {
            0x1 => Ok(FrameType::Headers(try!(HeadersFrame::from_raw(header, self)))),
            0x4 => Ok(FrameType::Settings(try!(SettingsFrame::from_raw(header, self)))),
            // TODO read and discard unknown frame payload
            _ => Ok(FrameType::Unknown),
        }
    }
}

/// All types implementing `io::Read` get `ReadFrame` by using the trait.
impl<R: Read> ReadFrame for R {}

pub trait WriteFrame: Write {
    fn write_frame<F: Frame>(&mut self, frame: F) -> Result<()> {
        self.write_all(frame.into().as_ref()).map_err(|e| From::from(e))
    }
}

impl<W: Write> WriteFrame for W {}

#[derive(Debug, Clone, PartialEq)]
pub struct Priority {
    exclusive: bool,
    dependency: u32,
    weight: u8,
}
