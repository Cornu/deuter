use connection::Connection;
use error::Result;
use frame::setting::SettingsFrame;
use frame::WriteFrame;

pub struct Client<C> {
    conn: C,
}

impl<C: Connection> Client<C> {
    // TODO default
    fn new(conn: C) -> Client<C> where C: Connection {
        Client {
            conn: conn,
        }
    }

    fn init(&mut self) -> Result<()> {
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        try!(self.conn.write(preface));

        // write settings frame
        let frame = SettingsFrame::ack();
        try!(self.conn.write_frame(frame));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;
    use mock::MockStream;
    use super::Client;
    use frame::{FrameType, ReadFrame};

    #[test]
    fn test_stream_init() {
        let (mut sconn, cconn) = MockStream::new();
        let mut client = Client::new(cconn);
        client.init();
        let mut buf = [0; 24];
        sconn.read(&mut buf).unwrap();
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        assert_eq!(&buf, preface);
        // settings frame
        if let FrameType::Settings(res) = sconn.read_frame(100).unwrap() {
            assert!(res.is_ack());
        } else {
            panic!("Wrong frame type")
        }
    }
}
