use std::collections::HashMap;
use std::io::{Read, Write};
use StreamId;
use frame::{Frame, FrameHeader, Flags, FLAG_PADDED, FLAG_PRIORITY, TYPE_HEADERS};
use frame::priority::PriorityFrame;
use error::{Error, ErrorKind, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderBlock {
    Decoded(Vec<Header>),
    Fragment(Vec<u8>)
}

impl<'a> Default for HeaderBlock {
    fn default() -> Self {
        HeaderBlock::Decoded(Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header(Vec<u8>, Vec<u8>);

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HeadersFrame {
    stream_id: StreamId,
    headers: HeaderBlock,
    priority: Option<PriorityFrame>,
    end_headers: bool,
    end_stream: bool,
}

impl HeadersFrame {
    pub fn read<R: Read>(header: FrameHeader, mut reader: R) -> Result<HeadersFrame> {
        if header.stream_id == 0 {
            return Err(Error::new(ErrorKind::Protocol, "Headers frames must be associated with a stream, stream id was zero"));
        }
        let mut frame: Self = Default::default();
        frame.stream_id = header.stream_id;

        let mut payload_len = header.payload_len;

        if header.flags.contains(FLAG_PADDED) {
            let mut buf = [0; 1];
            try!(reader.read_exact(&mut buf));
            let pad_len = buf[0] as usize;
            payload_len -= pad_len;
        }
        if header.flags.contains(FLAG_PRIORITY) {
            frame.priority = Some(try!(PriorityFrame::read(header, reader)));
        }
        let fragment = vec![0; payload_len];
        frame.headers = HeaderBlock::Fragment(fragment);
        // TODO read fragment
        Ok(frame)
    }

    fn set_stream_id(mut self, stream_id: StreamId) -> Self {
        self.stream_id = stream_id;
        self
    }
}

impl Frame for HeadersFrame {
    fn header(&self) -> FrameHeader {
        // TODO len and flags
        FrameHeader {
            payload_len: 0,
            frame_type: TYPE_HEADERS,
            flags: Flags::empty(),
            stream_id: self.stream_id,
        }
    }

    fn write<W: Write>(self, writer: &mut W) -> Result<()> {
        // TODO write headers
        if let Some(priority) = self.priority {
            //priority.write(writer);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{HeadersFrame, HeaderBlock};
    use StreamId;
    use frame::{ReadFrame, WriteFrame, FrameKind};

    #[test]
    fn test_trim_padding() {
        let buf = [2, 1, 2, 3, 4, 0, 0];
        //let new = trim_padding(buf.as_ref());
        //assert_eq!(new.len(), 4);
        //assert_eq!(new, [1, 2, 3, 4]);
    }

    #[test]
    fn test_empty_headers_frame() {
        let mut frame = HeadersFrame::default().set_stream_id(StreamId(1));
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
}

