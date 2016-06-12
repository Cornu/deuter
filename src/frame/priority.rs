use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use frame::{Frame, FrameHeader, FrameType, Flags};
use StreamId;
use error::{Error, Result};

pub const TYPE_PRIORITY : FrameType = 0x2;

const DEFAULT_WEIGHT : u8 = 16;
const PRIORITY_PAYLOAD_LENGTH : usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct PriorityFrame {
    stream_id: StreamId,
    exclusive: bool,
    dependency: StreamId,
    weight: u8,
}

impl PriorityFrame {
    pub fn new(stream_id: StreamId) -> Self {
        PriorityFrame {
            stream_id: stream_id,
            exclusive: false,
            dependency: StreamId(0),
            weight: DEFAULT_WEIGHT,
        }
    }

    fn dependency(mut self, stream_id: StreamId) -> Self {
        self.dependency = stream_id;
        self
    }

    fn exclusive(mut self) -> Self {
        self.exclusive = true;
        self
    }

    fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    pub fn read<R: Read>(header: FrameHeader, mut reader: R) -> Result<Self> {
        if header.stream_id == 0 {
            return Err(Error::protocol("Priority frame must be associated with a stream, stream id was zero"));
        }
        if header.payload_len != PRIORITY_PAYLOAD_LENGTH {
            return Err(Error::frame_size(format!("Bad payload length '{:?}'! The payload length for a priority frame must be 5 octets", header.payload_len)));
        }
        let mut buf = [0; PRIORITY_PAYLOAD_LENGTH];
        try!(reader.read_exact(&mut buf));
        let dep = BigEndian::read_u32(&mut buf);
        // Add one to the value to obtain a weight between 1 and 256 (section 6.3)
        let weight = buf[4] + 1;
        Ok(PriorityFrame {
            stream_id: header.stream_id,
            exclusive: dep & 0x80000000 != 0,
            dependency: dep.into(),
            weight: weight,
        })
    }

    fn write_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut buf = vec![0; PRIORITY_PAYLOAD_LENGTH];
        let mut dep = self.dependency.into();
        if self.exclusive {
            dep = dep | 0x80000000;
        }
        BigEndian::write_u32(&mut buf, dep);
        buf[4] = self.weight - 1;
        try!(writer.write(&buf));
        Ok(())
    }
}

impl Frame for PriorityFrame {
    fn header(&self) -> FrameHeader {
        FrameHeader {
            payload_len: PRIORITY_PAYLOAD_LENGTH,
            frame_type: TYPE_PRIORITY,
            flags: Flags::empty(),
            stream_id: self.stream_id,
        }
    }

    #[inline]
    fn write<W: Write>(self, writer: &mut W) -> Result<()> {
        self.write_payload(writer)
    }
}


#[cfg(test)]
mod test {
    use std::io::Cursor;
    use super::PriorityFrame;
    use frame::{ReadFrame, WriteFrame, FrameKind};
    use error::ErrorKind;

    #[test]
    fn test_default_frame() {
        let frame = PriorityFrame::new(1.into());
        assert!(!frame.is_exclusive());
        assert_eq!(frame.weight, 16);
        let mut b : Vec<u8> = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 5,     // length
                       2,           // type
                       0,           // flags
                       0, 0, 0, 1,  // stream id
                       0, 0, 0, 0,  // dependency
                       15]);        // weight
        let mut sl = &b[..];
        match sl.read_frame(100).unwrap() {
            FrameKind::Priority(f) => assert_eq!(frame, f),
            _ => panic!("Wrong frame type")
        };
    }

    #[test]
    fn test_exclusive_priority() {
        let frame = PriorityFrame::new(1.into()).exclusive();
        let mut b : Vec<u8> = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 5,      // length
                       2,            // type
                       0,            // flags
                       0, 0, 0, 1,   // stream id
                       128, 0, 0, 0, // dependency
                       15]);         // weight
        let mut sl = &b[..];
        match sl.read_frame(100).unwrap() {
            FrameKind::Priority(f) => assert_eq!(frame, f),
            _ => panic!("Wrong frame type")
        };
    }

    #[test]
    fn test_error_zero_stream() {
        let mut raw = Cursor::new([0, 0, 5,    // length
                                   2,          // type
                                   0,          // flags
                                   0, 0, 0, 0, // stream id
                                   0, 0, 0, 1, // dependency
                                   15]);       // weight
        assert_eq!(raw.read_frame(100).unwrap_err().kind(), ErrorKind::Protocol);
    }

    #[test]
    fn test_error_bad_size() {
        let mut raw = Cursor::new([0, 0, 6,    // length
                                   2,          // type
                                   0,          // flags
                                   0, 0, 0, 1, // stream id
                                   0, 0, 0, 1, // dependency
                                   15,         // weight
                                   0]);
        assert_eq!(raw.read_frame(100).unwrap_err().kind(), ErrorKind::FrameSize);
    }
}
