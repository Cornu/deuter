use std::io::{Read, Write};
use StreamId;
use frame::{Flags, Frame, FrameType, FrameHeader};
use error::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct UnknownFrame {
    stream_id: StreamId,
    flags: Flags,
    frame_type: FrameType,
    payload: Vec<u8>,
}

impl UnknownFrame {
    pub fn new(stream_id: StreamId,
               flags: Flags,
               frame_type: FrameType,
               payload: Vec<u8>)
               -> UnknownFrame {
        UnknownFrame {
            stream_id: stream_id,
            flags: flags,
            frame_type: frame_type,
            payload: payload,
        }
    }
}

impl Frame for UnknownFrame {
    fn from_reader<R: Read>(header: FrameHeader, mut reader: R) -> Result<UnknownFrame> {
        let mut payload = vec![0; header.payload_len];
        try!(reader.read_exact(&mut payload));
        Ok(UnknownFrame {
            stream_id: header.stream_id,
            flags: header.flags,
            frame_type: header.frame_type,
            payload: payload,
        })
    }

    fn into_writer<W: Write>(self, mut writer: W) -> Result<()> {
        writer.write_all(self.payload.as_ref()).map_err(|e| e.into())
    }

    fn payload_len(&self) -> usize {
        self.payload.len()
    }

    fn frame_type(&self) -> FrameType {
        self.frame_type
    }

    fn flags(&self) -> Flags {
        self.flags
    }

    fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

#[cfg(test)]
mod test {
    use super::UnknownFrame;
    use StreamId;
    use frame::{ReadFrame, WriteFrame, Flags, FrameKind};

    #[test]
    fn test_unknown_payload() {
        let frame = UnknownFrame::new(StreamId(1), Flags::empty(), 0xFF.into(), vec![1, 2, 3]);
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        let expected = vec![0, 0, 3,    // length
                            255,        // type
                            0,          // flags
                            0, 0, 0, 1, // stream id
                            1, 2, 3,    // data
                           ];
        assert_eq!(b, expected);
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Unknown(frame) => frame,
            _ => panic!("wrong frame"),
        };
        assert_eq!(frame, res);
    }
}
