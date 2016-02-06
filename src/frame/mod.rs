mod setting;

use std::io::{Read, Write};
use std::io;
use byteorder::{ByteOrder, BigEndian};
use self::setting::SettingsFrame;
use error::{Error, ConnectionError};

pub type Flags = u8;

trait Frame: Sized + Into<Vec<u8>> {
    fn payload_len(&self) -> usize;
    fn frame_type(&self) -> u8;
    fn flags(&self) -> u8;
    fn stream_id(&self) -> u32;
}

enum FrameType {
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
    fn read_frame(&mut self, max_size: usize) -> Result<FrameType, Error> {
        let mut buf = [0; 9];
        try!(self.read_exact(&mut buf));
        let payload_len = BigEndian::read_uint(&mut buf, 3) as usize;
        let frame_type = buf[3];
        let flags = buf[4];
        let stream_id = BigEndian::read_u32(&mut buf[5..]) & !0x80000000;

        if payload_len > max_size {
            return Err(Error::Connection(ConnectionError::FrameSize));
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
    fn write_frame<F: Frame>(&mut self, frame: F) -> Result<(), Error> {
        let mut buf = [0; 9];
        // write 24bit payload length
        BigEndian::write_uint(&mut buf, frame.payload_len() as u64, 3);
        buf[3] = frame.frame_type();
        buf[4] = frame.flags();
        BigEndian::write_u32(&mut buf[5..], frame.stream_id());
        try!(self.write_all(&buf[..]));
        try!(self.write_all(&mut frame.into()[..]));
        Ok(())
    }
}

impl<W: Write> WriteFrame for W {}
