extern crate byteorder;

#[cfg(test)]
mod mock;

mod error;
mod connection;
mod frame;
mod client;

use std::io::Read;
use connection::Connection;

use error::{Error, ErrorKind, Result};

pub struct Server<C> {
    conn: C,
    //max_frame_size: u32,
}

impl<C: Connection> Server<C> {
    fn new(conn: C) -> Server<C> where C: Connection {
        Server {
            conn: conn,
            //max_frame_size: 2^14,
        }
    }

    fn handle_preface(&mut self) -> Result<()> {
        let mut buf = [0; 24];
        try!(self.conn.read(&mut buf)); // TODO read_exact
        if &buf != b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" {
            return Err(Error::new(ErrorKind::Protocol, "bad preface"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};
    use super::mock::MockStream;
    use super::Server;
    use error::ErrorKind;

    #[test]
    fn test_tcpstream_connect() {
        let listener = TcpListener::bind("127.0.0.1:12345").unwrap();
        let _t = thread::spawn(move || {
            let mut conn = TcpStream::connect("127.0.0.1:12345").unwrap();
            conn.write(&[144]).unwrap();
        });

        let mut conn = listener.accept().unwrap().0;
        let mut buf = [0];
        conn.read(&mut buf).unwrap();
        assert!(buf[0] == 144);

    }

    #[test]
    fn handle_preface() {
        let (mut server, mut client) = MockStream::new();
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        client.write(preface).unwrap();
        let mut buf = [0; 24];
        assert_eq!(server.read(&mut buf).unwrap(), 24);
        assert_eq!(&buf, preface);
    }

    #[test]
    fn test_server_preface() {
        let (sconn, mut cconn) = MockStream::new();
        let mut server = Server::new(sconn);
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        cconn.write(preface).unwrap();
        server.handle_preface().unwrap();
    }

    #[test]
    fn test_worng_server_preface() {
        let (sconn, mut cconn) = MockStream::new();
        let mut server = Server::new(sconn);
        let preface = b"PRI * TTP/2.0\r\n\r\nSM\r\n\r\n";
        cconn.write(preface).unwrap();
        assert_eq!(server.handle_preface().unwrap_err().kind(), ErrorKind::Protocol);
    }
}
