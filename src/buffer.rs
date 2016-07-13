//! Buffer and Reader for Asynchronous / non-blocking IO

use std::io::{Read, ErrorKind};
use std::cmp;
use std::io::Result;
use std::ops::{Index, Range, RangeTo, RangeFrom, RangeFull};

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
            buf: vec![0; 64],
            start: 0,
            end: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.end == self.buf.len() {
            // reallocate
            //self.buf = Vec::with_capacity();
            let new_len = self.buf.len() + 64;
            self.buf.resize(new_len, 0);
        }
        // TODO read until WOULDBLOCK or read until the buffer was not fully filled
        self.end += try!(self.inner.read(&mut self.buf[self.end..]).or_else(|e| {
            match e.kind() {
                ErrorKind::WouldBlock => Ok(0),
                _ => Err(e),
            }
        }));
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = cmp::min(buf.len(), self.len());
        buf[..len].copy_from_slice(&self.buf[self.start..self.start+len]);
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

    fn index(&self, index: RangeFull) -> &[u8] {
        &self.buf[self.start..self.end]
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Write, Cursor};
    use std::net::{TcpListener, TcpStream};
    use super::AsyncBufReader;

    #[test]
    fn test_tcpstream_fillbuf() {
        let listener = TcpListener::bind("127.0.0.1:12345").unwrap();
        let mut tx = TcpStream::connect("127.0.0.1:12345").unwrap();

        let conn = listener.accept().unwrap().0;
        conn.set_nonblocking(true).unwrap();
        let mut r = AsyncBufReader::new(conn);
        assert_eq!(r.len(), 0);
        tx.write(&[1, 2, 3, 4]).unwrap();
        r.fill_buf();
        assert_eq!(r.len(), 4);
        let mut buf = [0; 6];
        r.read(&mut buf).unwrap();
        assert_eq!(buf, [1, 2 ,3, 4, 0, 0]);
        assert_eq!(r.len(), 0);
        tx.write(&[0; 20]).unwrap();
    }

    #[test]
    fn test_index() {
        let mut b = Cursor::new([1, 2, 3, 4, 5, 6]);
        let mut r = AsyncBufReader::new(b);
        r.fill_buf();
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
}
