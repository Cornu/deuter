pub mod headers;
pub mod priority;
pub mod settings;

use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use error::{Error, ErrorKind, Result};
use super::StreamId;
use self::settings::{SettingsFrame, TYPE_SETTINGS};
use self::headers::{HeadersFrame, TYPE_HEADERS};
use self::priority::{PriorityFrame, TYPE_PRIORITY};

pub type FrameType = u8;

pub const HEADER_SIZE: usize = 9;

bitflags! {
    #[derive(Default)] pub flags Flags: u8 {
        const FLAG_ACK         = 0x01,
        const FLAG_END_STREAM  = 0x01,
        const FLAG_END_HEADERS = 0x04,
        const FLAG_PADDED      = 0x08,
        const FLAG_PRIORITY    = 0x20,
    }
}

pub trait Frame: Sized {
    fn header(&self) -> FrameHeader;
    fn write<W: Write>(self, writer: &mut W) -> Result<()>;
}

#[derive(Debug)]
pub enum FrameKind {
    // Data,
    Headers(HeadersFrame),
    Priority(PriorityFrame),
    // RstConn,
    Settings(SettingsFrame),
    // PushPromise,
    // Ping,
    // GoAway,
    // WindowUpdate,
    // Continuation,
    // TODO remove 'Unknown', discard unknown frames or
    // better return Unknown Frame with raw payload
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameHeader {
    payload_len: usize,
    frame_type: FrameType,
    flags: Flags,
    stream_id: StreamId,
}

impl FrameHeader {
    fn read<R: Read>(mut reader: R) -> Result<FrameHeader> {
        let mut buf = [0; HEADER_SIZE];
        try!(reader.read_exact(&mut buf));
        Ok(FrameHeader {
            payload_len: BigEndian::read_uint(&mut buf, 3) as usize,
            frame_type: buf[3],
            flags: Flags::from_bits_truncate(buf[4]),
            stream_id: BigEndian::read_u32(&mut buf[5..]).into(),
        })
    }

    fn write<W: Write>(self, mut writer: W) -> Result<()> {
        let mut buf = [0; HEADER_SIZE];
        BigEndian::write_uint(&mut buf, self.payload_len as u64, 3);
        buf[3] = self.frame_type as u8;
        buf[4] = self.flags.bits();
        BigEndian::write_u32(&mut buf[5..], self.stream_id.into());
        try!(writer.write(buf.as_ref()));
        Ok(())
    }
}

pub trait ReadFrame: Read + Sized {
    fn read_frame(&mut self) -> Result<FrameKind> {
        self.read_frame_checked(usize::max_value())
    }

    fn read_frame_checked(&mut self, max_size: usize) -> Result<FrameKind> {
        // TODO use Read::take()
        let header = try!(FrameHeader::read(self.by_ref()));
        if header.payload_len > max_size {
            return Err(Error::new(ErrorKind::FrameSize,
                                  "payload length exceeds max frame size setting"));
        }
        match header.frame_type {
            TYPE_HEADERS => Ok(FrameKind::Headers(try!(HeadersFrame::read(header, self)))),
            TYPE_SETTINGS => Ok(FrameKind::Settings(try!(SettingsFrame::read(header, self)))),
            TYPE_PRIORITY => Ok(FrameKind::Priority(try!(PriorityFrame::read(header, self)))),
            // TODO read and discard unknown frame payload
            _ => Ok(FrameKind::Unknown),
        }
    }
}

/// All types implementing `io::Read` get `ReadFrame` by using the trait.
impl<R: Read> ReadFrame for R {}

pub trait WriteFrame: Write + Sized {
    fn write_frame<F: Frame>(&mut self, frame: F) -> Result<()> {
        try!(frame.header().write(self.by_ref()));
        try!(frame.write(self));
        Ok(())
    }
}

impl<W: Write> WriteFrame for W {}
