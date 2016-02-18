pub mod settings;

use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use self::settings::SettingsFrame;
use error::{Error, ErrorKind, Result};

pub type Flags = u8;

trait Frame: Sized + Into<Vec<u8>> {
    fn size(&self) -> usize;
}

#[derive(Debug)]
pub enum FrameType {
    //Data,
    //Headers,
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

pub trait ReadFrame: Read {
    fn read_frame(&mut self, max_size: usize) -> Result<FrameType> {
        let mut buf = [0; 9];
        try!(self.read_exact(&mut buf));
        let payload_len = BigEndian::read_uint(&mut buf, 3) as usize;
        let frame_type = buf[3];
        let flags = buf[4];
        let stream_id = BigEndian::read_u32(&mut buf[5..]) & !0x80000000;

        if payload_len > max_size {
            return Err(Error::new(ErrorKind::FrameSize, "payload length exceeds max frame size setting"));
        }
        let mut payload = vec![0; payload_len];
        try!(self.read_exact(&mut payload[..]));
        match frame_type {
            0x4 => Ok(FrameType::Settings(try!(SettingsFrame::from_raw(stream_id, flags, &payload[..])))),
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
