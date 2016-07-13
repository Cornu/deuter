use connection::Connection;
use error::{Error, ErrorKind, Result};
use frame::settings::{SettingsFrame, Setting};
use frame::{FrameKind, WriteFrame, ReadFrame};
use {Settings, WindowSize};

pub struct Client<C> {
    conn: C,
    settings: Settings,
    window_in: WindowSize,
    window_out: WindowSize,
}

impl<C: Connection> Client<C> {
    fn new(conn: C) -> Client<C>
        where C: Connection
    {
        Client {
            conn: conn,
            settings: Settings::default(),
            window_in: WindowSize::default(),
            window_out: WindowSize::default(),
        }
    }

    /// send preface string and settings frame
    // TODO: add settings
    fn send_preface(&mut self) -> Result<()> {
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        try!(self.conn.write(preface));

        // write settings frame
        let frame = SettingsFrame::default();
        try!(self.conn.write_frame(frame));

        Ok(())
    }

    /// check preface
    /// first received frame must be a valid settings frame
    fn receive_preface(&mut self) -> Result<()> {
        // read settings frame
        if let FrameKind::Settings(frame) = try!(self.conn.read_frame(100)) {
            if frame.is_ack() {
                return Err(Error::new(ErrorKind::Protocol, "Invalid Preface, received ACK"));
            }
        } else {
            return Err(Error::new(ErrorKind::Protocol, "Invalid Preface, bad frame"));
        }
        Ok(())
    }

    fn handle_next(&mut self) -> Result<()> {
        match try!(self.conn.read_frame(100)) {
            FrameKind::Settings(frame) => println!("Settings"),
            // Unknown
            _ => return Ok(()),
        }
        Ok(())
    }

    fn handle_settings_frame(&mut self, frame: SettingsFrame) {
        self.settings.update(frame)
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;
    use mock::MockStream;
    use super::Client;
    use frame::{FrameKind, ReadFrame, WriteFrame};
    use frame::settings::SettingsFrame;

    #[test]
    fn test_client_send_preface() {
        let (mut sconn, cconn) = MockStream::new();
        let mut client = Client::new(cconn);
        client.send_preface().unwrap();
        let mut buf = [0; 24];
        sconn.read(&mut buf).unwrap();
        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        assert_eq!(&buf, preface);
        // settings frame
        if let FrameKind::Settings(res) = sconn.read_frame(100).unwrap() {
            assert!(!res.is_ack());
        } else {
            panic!("Wrong frame type")
        }
    }

    #[test]
    fn test_client_receive_preface() {
        let (mut sconn, cconn) = MockStream::new();
        let mut client = Client::new(cconn);
        // server preface
        let frame = SettingsFrame::default();
        sconn.write_frame(frame).unwrap();
        client.receive_preface().unwrap();
    }
}
