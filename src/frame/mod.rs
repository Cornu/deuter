pub mod headers;
pub mod priority;
pub mod settings;
pub mod unknown;

use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use error::{Error, ErrorKind, Result};
use super::StreamId;
use self::settings::{SettingsFrame, TYPE_SETTINGS};
use self::headers::{HeadersFrame, TYPE_HEADERS};
use self::priority::{PriorityFrame, TYPE_PRIORITY};
use self::unknown::UnknownFrame;

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
    fn from_reader<R: Read>(header: FrameHeader, mut reader: R) -> Result<Self>;
    fn into_writer<W: Write>(self, mut writer: W) -> Result<()>;
    fn payload_len(&self) -> usize;
    fn frame_type(&self) -> FrameType;
    fn flags(&self) -> Flags {
        Flags::empty()
    }
    fn stream_id(&self) -> StreamId {
        StreamId(0)
    }
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
    Unknown(UnknownFrame),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameHeader {
    payload_len: usize,
    frame_type: FrameType,
    flags: Flags,
    stream_id: StreamId,
}

impl FrameHeader {
    fn new<F: Frame>(frame: &F) -> FrameHeader {
        FrameHeader {
            payload_len: frame.payload_len(),
            frame_type: frame.frame_type(),
            flags: frame.flags(),
            stream_id: frame.stream_id(),
        }
    }

    fn from_reader<R: Read>(mut reader: R) -> Result<FrameHeader> {
        let mut buf = [0; HEADER_SIZE];
        try!(reader.read_exact(&mut buf));
        Ok(FrameHeader {
            payload_len: BigEndian::read_uint(&mut buf, 3) as usize,
            frame_type: buf[3],
            flags: Flags::from_bits_truncate(buf[4]),
            stream_id: BigEndian::read_u32(&mut buf[5..]).into(),
        })
    }

    fn into_writer<W: Write>(self, mut writer: W) -> Result<()> {
        let mut buf = [0; HEADER_SIZE];
        BigEndian::write_uint(&mut buf, self.payload_len as u64, 3);
        buf[3] = self.frame_type as u8;
        buf[4] = self.flags.bits();
        BigEndian::write_u32(&mut buf[5..], self.stream_id.into());
        try!(writer.write_all(buf.as_ref()));
        Ok(())
    }
}

pub trait ReadFrame: Read + Sized {
    fn read_frame(&mut self) -> Result<FrameKind> {
        self.read_frame_checked(usize::max_value())
    }

    fn read_frame_checked(&mut self, max_size: usize) -> Result<FrameKind> {
        // TODO use Read::take()
        let header = try!(FrameHeader::from_reader(self.by_ref()));
        if header.payload_len > max_size {
            return Err(Error::new(ErrorKind::FrameSize,
                                  "payload length exceeds max frame size setting"));
        }
        match header.frame_type {
            TYPE_HEADERS => Ok(FrameKind::Headers(try!(HeadersFrame::from_reader(header, self)))),
            TYPE_SETTINGS => {
                Ok(FrameKind::Settings(try!(SettingsFrame::from_reader(header, self))))
            }
            TYPE_PRIORITY => {
                Ok(FrameKind::Priority(try!(PriorityFrame::from_reader(header, self))))
            }
            _ => Ok(FrameKind::Unknown(try!(UnknownFrame::from_reader(header, self)))),
        }
    }
}

/// All types implementing `io::Read` get `ReadFrame` by using the trait.
impl<R: Read> ReadFrame for R {}

pub trait WriteFrame: Write + Sized {
    fn write_frame<F: Frame>(&mut self, frame: F) -> Result<()> {
        try!(FrameHeader::new(&frame).into_writer(self.by_ref()));
        try!(frame.into_writer(self));
        Ok(())
    }
}

impl<W: Write> WriteFrame for W {}

/// Iterate over a slice of bytes yielding Frames
pub struct FrameIter<'a> {
    buf: &'a [u8],
    pos: usize,
    max_payload: usize,
}

impl<'a> FrameIter<'a> {
    pub fn new(buf: &[u8], max: usize) -> FrameIter {
        FrameIter {
            buf: buf,
            pos: 0,
            max_payload: max,
        }
    }

    fn len(&self) -> usize {
        self.buf.len() - self.pos
    }
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Result<FrameKind>;

    fn next(&mut self) -> Option<Result<FrameKind>> {
        if self.len() < 3 {
            return None;
        }
        let mut buf = &self.buf[self.pos..];
        let payload_len = BigEndian::read_uint(&buf[..4], 3) as usize;
        if payload_len > self.max_payload {
            return Some(Err(Error::new(ErrorKind::FrameSize,
                                       "payload length exceeds max frame size setting")));
        }
        let size = payload_len + HEADER_SIZE;
        if self.len() < size {
            return None;
        }
        self.pos += size;
        Some(buf.read_frame())
    }
}

#[cfg(test)]
mod test {
    use super::FrameIter;
    use frame::{FrameKind, Frame};
    use error::ErrorKind;

    #[test]
    fn test_iter_empty_slice() {
        assert!(FrameIter::new(&[], 100).next().is_none());
    }

    #[test]
    fn test_iter_incomplete_frame_slice() {
        assert!(FrameIter::new(&[0, 0, 0, 1], 100).next().is_none());
    }

    #[test]
    fn test_iter_complete_frame_slice() {
        let f = vec![0, 0, 4,     // length
                     1,           // type headers
                     0,           // flags
                     0, 0, 0, 1,  // stream id
                     0, 1, 2, 3,  // fragment
                    ];
        let mut iter = FrameIter::new(&f, 100);
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_multiple_frames() {
        let f = vec![0, 0, 4,     // length
                     1,           // type headers
                     0,           // flags
                     0, 0, 0, 1,  // stream id
                     0, 1, 2, 3,  // fragment
                                  // ---
                     0, 0, 3,     // length
                     11,          // type headers
                     0,           // flags
                     0, 0, 0, 2,  // stream id
                     3, 2, 1,     // fragment
                    ];
        let mut iter = FrameIter::new(&f, 100);
        let frame1 = match iter.next().unwrap().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame"),
        };
        assert_eq!(frame1.stream_id(), 1);
        let frame2 = match iter.next().unwrap().unwrap() {
            FrameKind::Unknown(frame) => frame,
            _ => panic!("Wrong frame"),
        };
        assert_eq!(frame2.stream_id(), 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_max_payload_error() {
        assert_eq!(FrameIter::new(&[0, 0, 210, 1], 100).next().unwrap().err().unwrap().kind(),
                   ErrorKind::FrameSize);
    }
}
