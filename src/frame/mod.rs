mod setting;

use std::io::{Read, Write};
use std::io;
use byteorder::{ByteOrder, BigEndian};
use self::setting::SettingsFrame;

trait Frame: Sized {
    fn from_raw(stream_id: u32, flags: u8, payload: &[u8]) -> ::Result<Self>;
    fn payload_len(&self) -> usize;
    fn frame_type(&self) -> u8;
    fn flags(&self) -> u8;
    fn stream_id(&self) -> u32;
}

enum FrameType {
    Data,
    Headers,
    Priority,
    RstConn,
    Settings(SettingsFrame),
    PushPromise,
    Ping,
    GoAway,
    WindowUpdate,
    Continuation,
}

pub trait ReadFrame: Read {
    fn read_frame(&mut self, max_size: usize) -> ::Result<FrameType> {
        let mut buf = [0; 9];
        self.read_exact(&mut buf);
        let payload_len = BigEndian::read_uint(&mut buf, 3) as usize;
        let frame_type = buf[3];
        let flags = buf[4];
        let stream_id = BigEndian::read_u32(&mut buf[5..]) & !0x80000000;

        if payload_len > max_size {
            return Err(::Error::FrameSize);
        }
        let payload = Vec::with_capacity(payload_len);
        match frame_type {
            0x4 => Ok(FrameType::Settings(try!(SettingsFrame::from_raw(stream_id, flags, &payload[..])))),
            _ => Err(::Error::Protocol)
        }
    }
}

/// All types implementing `io::Read` get `ReadFrame` by using the trait.
impl<R: Read> ReadFrame for R {}

pub trait WriteFrame: Write {
    fn write_frame<F: Frame>(&mut self, frame: F) -> ::Result<()> {
        let mut buf = [0;9];
        BigEndian::write_uint(&mut buf, frame.payload_len() as u64, 3);
        buf[3] = frame.frame_type();
        buf[4] = frame.flags();
        BigEndian::write_u32(&mut buf[5..], frame.stream_id());
        self.write_all(&buf[..]);
        Ok(())
    }
}

impl<W: Write> WriteFrame for W {}
