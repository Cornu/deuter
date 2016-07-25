//! Buffer and Reader for Asynchronous / non-blocking IO

use std::io;
use std::io::{Read, BufRead, ErrorKind};
use std::cmp;
use std::ops::{Index, Range, RangeTo, RangeFrom, RangeFull};

use frame::FrameIter;
use error::Result;

const INITIAL_BUF_SIZE: usize = 64;
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// The `AsyncBufReader` adds asynchronous buffering to any reader.
///
/// contiguous growable, sliding buffer
///
/// ```
/// use std::net::{TcpListener, TcpStream};
/// use std::io::{Read, Write, BufRead};
/// use deuter::buffer::AsyncBufReader;
///
/// let listener = TcpListener::bind("127.0.0.1:12345").unwrap();
/// let mut tx = TcpStream::connect("127.0.0.1:12345").unwrap();
///
/// let conn = listener.accept().unwrap().0;
/// conn.set_nonblocking(true).unwrap();
/// let mut r = AsyncBufReader::new(conn);
/// assert_eq!(r.len(), 0);
/// tx.write(&[1, 2, 3, 4]).unwrap();
/// r.fill_buf();
/// assert_eq!(r.len(), 4);
/// let mut buf = [0; 6];
/// r.read(&mut buf).unwrap();
/// assert_eq!(buf, [1, 2 ,3, 4, 0, 0]);
/// assert_eq!(r.len(), 0);
/// ```
///
/// `AsyncBufReader` implements `Index` to peek into the buffer.
pub struct AsyncBufReader<R> {
    inner: R,
    buf: Vec<u8>,
    pos: usize,
    cap: usize,
}

impl<R: Read> AsyncBufReader<R> {
    pub fn new(inner: R) -> AsyncBufReader<R> {
        AsyncBufReader {
            inner: inner,
            buf: vec![0; INITIAL_BUF_SIZE],
            pos: 0,
            cap: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.cap - self.pos
    }
}

impl<R: Read> Read for AsyncBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self.buf[self.pos..self.pos + len]);
        self.consume(len);
        Ok(len)
    }
}

// TODO check other `BufRead` trait functions to work with our `fill_buf()`
impl<R: Read> BufRead for AsyncBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        loop {
            if self.cap == self.buf.len() {
                // double the allocated space, for small sizes,
                // else allocated extra DEFAULT_BUF_SIZE
                let new_len = self.len() + cmp::min(self.len(), DEFAULT_BUF_SIZE);
                let mut new_buf = vec![0; new_len];
                new_buf.copy_from_slice(&self[..]);
                self.buf = new_buf;
            }
            let remaining = self.buf.len() - self.cap;
            let nread = try!(self.inner.read(&mut self.buf[self.cap..]).or_else(|e| {
                match e.kind() {
                    ErrorKind::WouldBlock => Ok(0),
                    _ => Err(e),
                }
            }));
            self.cap += nread;
            // if we read exactly until our buffer is full, there could be more data
            // else break here
            if nread != remaining {
                break;
            }
        }
        Ok(&self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
        // if we consumed everything until the end, reset buffer to beginning
        if self.pos == self.cap {
            self.pos = 0;
            self.cap = 0;
        }
    }
}

impl<R: Read> Index<usize> for AsyncBufReader<R> {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 {
        &self.buf[self.pos + index]
    }
}

impl<R: Read> Index<Range<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &[u8] {
        &self.buf[self.pos + index.start..self.pos + index.end]
    }
}

impl<R: Read> Index<RangeTo<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &[u8] {
        &self.buf[self.pos..self.pos + index.end]
    }
}

impl<R: Read> Index<RangeFrom<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &[u8] {
        &self.buf[self.pos + index.start..self.cap]
    }
}

impl<R: Read> Index<RangeFull> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, _index: RangeFull) -> &[u8] {
        &self.buf[self.pos..self.cap]
    }
}

// #############

pub struct FrameReader<R> {
    inner: AsyncBufReader<R>,
    max_payload: usize,
}

impl<'a, R: Read> FrameReader<R> {
    fn new(inner: R, max_payload: usize) -> FrameReader<R> {
        FrameReader {
            inner: AsyncBufReader::new(inner),
            max_payload: max_payload,
        }
    }

    fn frames(&'a mut self) -> Result<FrameIter<'a>> {
        let buf = try!(self.inner.fill_buf());
        Ok(FrameIter::new(buf, self.max_payload))
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write, BufRead, Cursor};
    use std::net::{TcpListener, TcpStream};
    use super::{AsyncBufReader, FrameReader};
    use StreamId;
    use frame::{Frame, WriteFrame, FrameKind};
    use frame::headers::HeadersFrame;

    #[test]
    fn test_tcpstream_fillbuf() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut tx = TcpStream::connect(listener.local_addr().unwrap()).unwrap();

        let conn = listener.accept().unwrap().0;
        conn.set_nonblocking(true).unwrap();
        let mut r = AsyncBufReader::new(conn);
        assert_eq!(r.len(), 0);
        tx.write(&[1, 2, 3, 4]).unwrap();
        r.fill_buf().unwrap();
        assert_eq!(r.len(), 4);
        let mut buf = [0; 6];
        r.read(&mut buf).unwrap();
        assert_eq!(buf, [1, 2, 3, 4, 0, 0]);
        assert_eq!(r.len(), 0);
        tx.write(&[0; 20]).unwrap();
    }

    #[test]
    fn test_index() {
        let b = Cursor::new([1, 2, 3, 4, 5, 6]);
        let mut r = AsyncBufReader::new(b);
        r.fill_buf().unwrap();
        assert_eq!(r[0], 1);
        assert_eq!(r[1..3], [2, 3]);
        assert_eq!(r[..3], [1, 2, 3]);
        assert_eq!(r[1..], [2, 3, 4, 5, 6]);
        let mut d = [0; 2];
        r.read(&mut d).unwrap();
        assert_eq!(r[0], 3);
        assert_eq!(r[1..3], [4, 5]);
        assert_eq!(r[..3], [3, 4, 5]);
        assert_eq!(r[1..], [4, 5, 6]);
    }

    #[test]
    fn test_iter_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut tx = TcpStream::connect(listener.local_addr().unwrap()).unwrap();

        let conn = listener.accept().unwrap().0;
        conn.set_nonblocking(true).unwrap();
        let mut r = FrameReader::new(conn, 100);

        assert!(r.frames().unwrap().next().is_none());
        tx.write_frame(HeadersFrame::new(StreamId(1))).unwrap();
        tx.write_frame(HeadersFrame::new(StreamId(2))).unwrap();
        let mut iter = r.frames().unwrap();
        let frame1 = match iter.next().unwrap().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame"),
        };
        assert_eq!(frame1.stream_id(), 1);
        let frame2 = match iter.next().unwrap().unwrap() {
            FrameKind::Headers(frame) => frame,
            _ => panic!("Wrong frame"),
        };
        assert_eq!(frame2.stream_id(), 2);
        assert!(iter.next().is_none());
    }
}
