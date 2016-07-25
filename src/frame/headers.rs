use std::io::{Read, Write};
use StreamId;
use frame::{Frame, FrameHeader, FrameType, Flags, FLAG_PADDED, FLAG_PRIORITY, FLAG_END_HEADERS,
            FLAG_END_STREAM};
use frame::priority::{PriorityFrame, PRIORITY_PAYLOAD_LENGTH};
use error::{Error, Result};

pub const TYPE_HEADERS: FrameType = 0x1;

#[derive(Debug, Clone, PartialEq)]
pub struct HeadersFrame {
    stream_id: StreamId,
    fragment: Vec<u8>,
    priority: Option<PriorityFrame>,
    end_headers: bool,
    end_stream: bool,
}

impl HeadersFrame {
    pub fn new(stream_id: StreamId) -> Self {
        HeadersFrame {
            stream_id: stream_id,
            fragment: Vec::new(),
            priority: None,
            end_headers: false,
            end_stream: false,
        }
    }

    fn priority(mut self, priority: PriorityFrame) -> Self {
        self.priority = Some(priority);
        self
    }

    fn fragment<T: Into<Vec<u8>>>(mut self, fragment: T) -> Self {
        self.fragment = fragment.into();
        self
    }

    fn end_headers(mut self) -> Self {
        self.end_headers = true;
        self
    }

    fn end_stream(mut self) -> Self {
        self.end_stream = true;
        self
    }
}

impl Frame for HeadersFrame {
    fn from_reader<R: Read>(header: FrameHeader, mut reader: R) -> Result<HeadersFrame> {
        if header.stream_id == 0 {
            return Err(Error::protocol("Headers frame must be associated with a stream, stream \
                                        id was zero"));
        }

        let mut payload_len = header.payload_len;

        let mut pad_len = 0;
        if header.flags.contains(FLAG_PADDED) {
            let mut buf = [0; 1];
            try!(reader.read_exact(&mut buf));
            pad_len = buf[0] as usize;
            payload_len -= pad_len + 1;
        }

        let mut priority = None;
        if header.flags.contains(FLAG_PRIORITY) {
            priority = Some(try!(PriorityFrame::from_reader(header.clone(), reader.by_ref())));
            payload_len -= PRIORITY_PAYLOAD_LENGTH;
        }

        let mut fragment = vec![0; payload_len];
        try!(reader.read_exact(&mut fragment));

        // read, discard padding
        if header.flags.contains(FLAG_PADDED) {
            let mut padding = vec![0; pad_len];
            try!(reader.read_exact(&mut padding));
        }

        Ok(HeadersFrame {
            stream_id: header.stream_id,
            fragment: fragment,
            priority: priority,
            end_headers: header.flags.contains(FLAG_END_HEADERS),
            end_stream: header.flags.contains(FLAG_END_STREAM),
        })
    }

    fn into_writer<W: Write>(self, mut writer: W) -> Result<()> {
        // TODO padding
        if let Some(priority) = self.priority {
            try!(priority.into_writer(writer.by_ref()));
        }
        try!(writer.write_all(self.fragment.as_ref()));
        Ok(())
    }

    fn payload_len(&self) -> usize {
        let mut len = self.fragment.len();
        if let Some(_) = self.priority {
            len += PRIORITY_PAYLOAD_LENGTH;
        }
        len
    }

    fn frame_type(&self) -> FrameType {
        TYPE_HEADERS
    }

    fn flags(&self) -> Flags {
        let mut flags = Flags::empty();
        if self.end_headers {
            flags.insert(FLAG_END_HEADERS);
        }
        if self.end_stream {
            flags.insert(FLAG_END_STREAM);
        }
        if let Some(_) = self.priority {
            flags.insert(FLAG_PRIORITY);
        }
        flags
    }

    fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;
    use super::HeadersFrame;
    use StreamId;
    use frame::{ReadFrame, WriteFrame, FrameKind};
    use frame::priority::PriorityFrame;

    #[test]
    fn test_empty_headers_frame() {
        let frame = HeadersFrame::new(StreamId(1));
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 0, 1, 0, 0, 0, 0, 1]);
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type"),
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_priority_headers_frame() {
        let priority = PriorityFrame::new(StreamId(1));
        let frame = HeadersFrame::new(StreamId(1)).priority(priority);
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        let expected = vec![0, 0, 5,    // length
                            1,          // type
                            0x20,       // flags
                            0, 0, 0, 1, // stream id
                            0, 0, 0, 0, // dependency
                            15,         // weight
                           ];
        assert_eq!(b, expected);
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type"),
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_fragment_in_headers_frame() {
        let fragment = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let frame = HeadersFrame::new(StreamId(1)).fragment(fragment);
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        let expected = vec![0, 0, 10,   // length
                            1,          // type
                            0,          // flags
                            0, 0, 0, 1, // stream id
                            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, // fragment
                           ];
        assert_eq!(b, expected);
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type"),
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_flags_in_headers_frame() {
        let frame = HeadersFrame::new(StreamId(1)).end_headers().end_stream();
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        let expected = vec![0, 0, 0,    // length
                            1,          // type
                            5,          // flags
                            0, 0, 0, 1, // stream id
                           ];
        assert_eq!(b, expected);
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type"),
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_padding_headers_frame() {
        let b = vec![0, 0, 8,          // length
                     1,                // type
                     8,                // flags
                     0, 0, 0, 1,       // stream id
                     3,                // padding length
                     0, 1, 2, 3,       // fragment
                     0xFF, 0xFF, 0xFF, // padding
                     4, 4, 4, 4, 4,    // make sure we can read further
                    ];
        let mut sl = &b[..];
        let res = match sl.read_frame().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame type"),
        };
        assert_eq!(res.fragment, [0, 1, 2, 3]);
        let mut buf = [0; 4];
        sl.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [4, 4, 4, 4]);
    }
}
