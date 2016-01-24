use std::net::TcpStream;
use std::io::{Read, Write};
use std::io;
use byteorder::{ByteOrder, BigEndian};

pub trait Connection: Read + Write {
    #[inline]
    fn read_u24(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        try!(self.read_exact(&mut buf[1..]));
        Ok(BigEndian::read_u32(&buf))
    }

    #[inline]
    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        try!(self.read_exact(&mut buf));
        Ok(BigEndian::read_u32(&buf))
    }

    #[inline]
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        try!(self.read_exact(&mut buf));
        Ok(buf[0])
    }
}

impl Connection for TcpStream {}

#[cfg(test)]
mod test {
    use std::io::{Read, Write};
    use mock::MockStream;
    use super::Connection;

    #[test]
    fn test_read_u24() {
        let (mut server, mut client) = MockStream::new();
        assert_eq!(client.write(&[2, 5, 3, 10, 6, 63, 15]).unwrap(), 7);
        assert_eq!(server.read_u24().unwrap(), 132355);
        assert_eq!(server.read_u32().unwrap(), 168181519);
    }
}
