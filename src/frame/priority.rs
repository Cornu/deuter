use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use frame::{Frame, FrameHeader, Flags, TYPE_PRIORITY};
use StreamId;
use error::{Error, Result};

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

    pub fn read<R: Read>(header: FrameHeader, reader: R) -> Result<Self> {
        if header.stream_id != 0 {
            return Err(Error::protocol(format!("Bad StreamId '{:?}'! The stream identifier for a priority frame must be zero", header.stream_id)));
        }
        if header.payload_len != PRIORITY_PAYLOAD_LENGTH {
            return Err(Error::frame_size(format!("Bad payload length '{:?}'! The payload length for a priority frame must be 5 octets", header.payload_len)));
        }
        let mut buf = [0; PRIORITY_PAYLOAD_LENGTH];
        let dep = BigEndian::read_u32(&mut buf);
        // Add one to the value to obtain a weight between 1 and 256 (section 6.3)
        let weight = buf[4] + 1;
        Ok(PriorityFrame {
            stream_id: header.stream_id,
            exclusive: dep & 0x8000000 != 0,
            dependency: dep.into(),
            weight: weight,
        })
    }

    fn write_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut buf = vec![0; PRIORITY_PAYLOAD_LENGTH];
        let mut dep = self.stream_id.into();
        if self.exclusive {
            dep = dep & 0x80000000;
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
    use super::PriorityFrame;
    use frame::{ReadFrame, WriteFrame, FrameKind};

    #[test]
    fn test_default_priority_frame() {
        let frame = PriorityFrame::new(1.into());
        let mut b : Vec<u8> = Vec::new();
        b.write_frame(frame).unwrap();
        assert_eq!(b, [0, 0, 5,     // length
                       2,           // type
                       0,           // flags
                       0, 0, 0, 1,  // stream id
                       0, 0, 0, 1,  // dependency
                       15]);        // weight
    }
}
