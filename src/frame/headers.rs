use std::collections::HashMap;
use std::io::{Read, Write};
use StreamId;
use frame::{Frame, FrameHeader, FrameType, Flags, FLAG_PADDED, FLAG_PRIORITY, FLAG_END_HEADERS, FLAG_END_STREAM};
use frame::priority::{PriorityFrame, PRIORITY_PAYLOAD_LENGTH};
use error::{Error, ErrorKind, Result};

pub const TYPE_HEADERS  : FrameType = 0x1;

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderBlock {
    Decoded(Vec<Header>),
    Fragment(Vec<u8>)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header(Vec<u8>, Vec<u8>);

#[derive(Debug, Clone, PartialEq)]
pub struct HeadersFrame {
    stream_id: StreamId,
    headers: HeaderBlock,
    priority: Option<PriorityFrame>,
    end_headers: bool,
    end_stream: bool,
}

impl HeadersFrame {
    pub fn new(stream_id: StreamId) -> Self {
        HeadersFrame {
            stream_id: stream_id,
            headers: HeaderBlock::Decoded(Vec::new()),
            priority: None,
            end_headers: false,
            end_stream: false,
        }
    }

    fn priority(mut self, priority: PriorityFrame) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn read<R: Read>(header: FrameHeader, mut reader: R) -> Result<HeadersFrame> {
        if header.stream_id == 0 {
            return Err(Error::protocol("Headers frame must be associated with a stream, stream id was zero"));
        }

        let mut payload_len = header.payload_len;

        if header.flags.contains(FLAG_PADDED) {
            let mut buf = [0; 1];
            try!(reader.read_exact(&mut buf));
            let pad_len = buf[0] as usize;
            payload_len -= pad_len;
        }

        let mut priority = None;
        if header.flags.contains(FLAG_PRIORITY) {
            priority = Some(try!(PriorityFrame::read(header.clone(), reader.by_ref())));
            payload_len -= PRIORITY_PAYLOAD_LENGTH;
        }

        let mut fragment = vec![0; payload_len];
        try!(reader.read_exact(&mut fragment));
        // TODO read, discard padding
        Ok(HeadersFrame {
            stream_id: header.stream_id,
            headers: HeaderBlock::Fragment(fragment),
            priority: priority,
            end_headers: header.flags.contains(FLAG_END_HEADERS),
            end_stream: header.flags.contains(FLAG_END_STREAM),
        })
    }
}

impl Frame for HeadersFrame {
    fn header(&self) -> FrameHeader {
        // TODO len and flags
        let mut len = 0;
        let mut flags = Flags::empty();
        if let Some(_) = self.priority {
            flags.insert(FLAG_PRIORITY);
            len += PRIORITY_PAYLOAD_LENGTH;
        }
        FrameHeader {
            payload_len: len,
            frame_type: TYPE_HEADERS,
            flags: flags,
            stream_id: self.stream_id,
        }
    }

    fn write<W: Write>(self, writer: &mut W) -> Result<()> {
        // TODO write headers
        if let Some(priority) = self.priority {
            priority.write(writer);
        }
        match self.headers {
            HeaderBlock::Fragment(fragment) => try!(writer.write(fragment.as_ref())),
            _ => return Err(Error::internal("Unencoded Header Fragment, must be encoded")),
        };
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{HeadersFrame, HeaderBlock};
    use StreamId;
    use frame::{ReadFrame, WriteFrame, FrameKind, FLAG_PRIORITY};
    use frame::priority::PriorityFrame;

    #[test]
    fn test_empty_headers_frame() {
        let mut frame = HeadersFrame::new(StreamId(1));
        frame.headers = HeaderBlock::Fragment(Vec::new());
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 0, 1, 0, 0, 0, 0, 1]);
        let mut sl = &b[..];
        let res = match sl.read_frame(100).unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type")
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_priority_headers_frame() {
        let priority = PriorityFrame::new(StreamId(1));
        let mut frame = HeadersFrame::new(StreamId(1)).priority(priority);
        frame.headers = HeaderBlock::Fragment(Vec::new());
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 5,       // length
                       1,             // type
                       0x20,          // flags
                       0, 0, 0, 1,    // stream id
                       0, 0, 0, 0,    // dependency
                       15]);          // weight
        let mut sl = &b[..];
        let res = match sl.read_frame(100).unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type")
        };
        assert_eq!(frame, res);
    }
}

