use std::io::{Read, Write};
use std::net::TcpStream;

pub trait Connection: Read + Write {
}

impl Connection for TcpStream {}

#[cfg(test)]
mod test {
    //use mock::MockStream;
    //use super::Connection;

    #[test]
    fn test_read_u24() {
        //let (mut server, mut client) = MockStream::new();
    }
}
