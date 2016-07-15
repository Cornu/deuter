//! Buffer and Reader for Asynchronous / non-blocking IO

use std::io;
use std::io::{Read, ErrorKind};
use std::cmp;
use std::ops::{Index, Range, RangeTo, RangeFrom, RangeFull};
use byteorder::{ByteOrder, BigEndian};

use frame::{ReadFrame, FrameKind, HEADER_SIZE};
use error::Result;

const INITIAL_BUF_SIZE: usize = 64;
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// The `AsyncBufReader` adds asynchronous buffering to any reader.
///
/// ```
/// use std::net::{TcpListener, TcpStream};
/// use std::io::{Read, Write};
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
    start: usize,
    end: usize,
}

impl<R: Read> AsyncBufReader<R> {
    pub fn new(inner: R) -> AsyncBufReader<R> {
        AsyncBufReader {
            inner: inner,
            buf: vec![0; INITIAL_BUF_SIZE],
            start: 0,
            end: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn fill_buf(&mut self) -> io::Result<&[u8]> {
        loop {
            if self.end == self.buf.len() {
                // double the allocated space, for small sizes,
                // else allocated extra DEFAULT_BUF_SIZE
                let new_len = self.len() + cmp::min(self.len(), DEFAULT_BUF_SIZE);
                let mut new_buf = vec![0; new_len];
                new_buf.copy_from_slice(&self[..]);
                self.buf = new_buf;
            }
            let remaining = self.buf.len() - self.end;
            let nread = try!(self.inner.read(&mut self.buf[self.end..]).or_else(|e| {
                match e.kind() {
                    ErrorKind::WouldBlock => Ok(0),
                    _ => Err(e),
                }
            }));
            self.end += nread;
            // if we read exactly until our buffer is full, there could be more data
            // else break here
            if nread != remaining {
                break;
            }
        }
        Ok(&self.buf[self.start..self.end])
    }

    fn consume(&mut self, amt: usize) {
        self.start = cmp::min(self.start + amt, self.end);
        // if we consumed everything until the end, reset buffer to start
        if self.start == self.end {
            self.start = 0;
            self.end = 0;
        }
    }
}

impl<R: Read> Read for AsyncBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self.buf[self.start..self.start + len]);
        self.consume(len);
        Ok(len)
    }
}

impl<R: Read> Index<usize> for AsyncBufReader<R> {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 {
        &self.buf[self.start + index]
    }
}

impl<R: Read> Index<Range<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &[u8] {
        &self.buf[self.start + index.start..self.start + index.end]
    }
}

impl<R: Read> Index<RangeTo<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &[u8] {
        &self.buf[self.start..self.start + index.end]
    }
}

impl<R: Read> Index<RangeFrom<usize>> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &[u8] {
        &self.buf[self.start + index.start..self.end]
    }
}

impl<R: Read> Index<RangeFull> for AsyncBufReader<R> {
    type Output = [u8];

    fn index(&self, _index: RangeFull) -> &[u8] {
        &self.buf[self.start..self.end]
    }
}

// #############

pub trait AsyncReadFrame {
    fn try_read_frame(&mut self) -> Result<Option<FrameKind>>;
}

impl<R: Read> AsyncReadFrame for AsyncBufReader<R> {
    fn try_read_frame(&mut self) -> Result<Option<FrameKind>> {
        try!(self.fill_buf());
        // header starts with 24 bit length field
        if self.len() < 3 {
            return Ok(None);
        }
        let size = BigEndian::read_uint(&self[..4], 3) as usize;
        if self.len() < size + HEADER_SIZE {
            return Ok(None);
        }
        self.read_frame().map(|f| Some(f))
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write, Cursor};
    use std::net::{TcpListener, TcpStream};
    use super::{AsyncBufReader, AsyncReadFrame};

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
        let mut b = Cursor::new([1, 2, 3, 4, 5, 6]);
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
    fn test_read_frame() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut tx = TcpStream::connect(listener.local_addr().unwrap()).unwrap();

        let conn = listener.accept().unwrap().0;
        conn.set_nonblocking(true).unwrap();
        let mut r = AsyncBufReader::new(conn);

        assert!(r.try_read_frame().unwrap().is_none());

        let b = vec![0, 0, 0,     // length
                     1,           // type headers
                     0,           // flags
                     0, 0, 0, 1,  // stream id
                     0, 1, 2, 3,  // fragment
                    ];
        tx.write(&b[0..4]).unwrap();
        r.fill_buf().unwrap();
        assert!(r.try_read_frame().unwrap().is_none());
        tx.write(&b[4..]).unwrap();
        r.fill_buf().unwrap();
        assert_eq!(r.len(), 13);
        assert!(r.try_read_frame().unwrap().is_some());
    }
}
