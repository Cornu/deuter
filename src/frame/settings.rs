use byteorder::{ByteOrder, BigEndian};
use std::io::{Read, Write};
use StreamId;
use frame::{Frame, FrameHeader, Flags, FLAG_ACK, TYPE_SETTINGS};
use error::{Error, ErrorKind, Result};

const MAX_FLOW_CONTROL_WINDOW_SIZE : u32 = ::std::i32::MAX as u32;
const MIN_FRAME_SIZE : u32 = 16384;
const MAX_FRAME_SIZE : u32 = 16777215;

// each settings consists of an 2 byte identifier and 4 byte value
const SETTING_LENGTH : usize = 6;

/// Settings Parameter according to rfc 6.5.2
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    HeaderTableSize(u32),
    EnablePush(bool),
    MaxConcurrentStreams(u32),
    InitialWindowSize(i32),
    MaxFrameSize(u32),
    MaxHeaderListSize(u32)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SettingsFrame {
    settings: Vec<Setting>,
    ack: bool,
}

impl SettingsFrame {
    pub fn read<R: Read>(header: FrameHeader, mut reader: R) -> Result<SettingsFrame> {
        if header.stream_id != 0 {
            return Err(Error::new(ErrorKind::Protocol, "The stream identifier for a settings frame must be zero"));
        }
        let mut frame: Self = Default::default();
        if header.flags.contains(FLAG_ACK) {
            frame.ack = true;
            if header.payload_len != 0 {
                return Err(Error::new(ErrorKind::FrameSize, "Settings Frame with Ack Flag must be empty"));
            }
        }
        if header.payload_len % 6 != 0 {
            return Err(Error::new(ErrorKind::FrameSize, "Settings Frame payload length must be multiple of 6"));
        }
        for _ in 0..header.payload_len / 6 {
            try!(frame.read_setting(reader.by_ref()));
        }
        Ok(frame)
    }

    pub fn ack() -> Self {
        Self::default().set_ack()
    }

    fn set_ack(mut self) -> Self {
        self.ack = true;
        self
    }

    #[inline]
    pub fn add_setting(&mut self, set: Setting) {
        self.settings.push(set);
    }

    #[inline]
    pub fn is_ack(&self) -> bool {
        self.ack
    }

    #[inline]
    pub fn settings(self) -> Vec<Setting> {
        self.settings
    }

    fn read_setting<R: Read>(&mut self, mut reader: R) -> Result<()> {
        let mut buf = [0; 6];
        try!(reader.read_exact(&mut buf));
        let id = BigEndian::read_u16(&buf);
        let val = BigEndian::read_u32(&buf[2..]);
        // parse according to rfc 6.5.2
        match id {
            0x1 => self.settings.push(Setting::HeaderTableSize(val)),
            0x2 => match val {
                0 => self.settings.push(Setting::EnablePush(false)),
                1 => self.settings.push(Setting::EnablePush(true)),
                _ => return Err(Error::new(ErrorKind::Protocol, "Invalid Value for enable push setting in settings frame")),
            },
            0x3 => self.settings.push(Setting::MaxConcurrentStreams(val)),
            0x4 => {
                if val > MAX_FLOW_CONTROL_WINDOW_SIZE {
                    return Err(Error::new(ErrorKind::FlowControl, "Initial window size must be lower than 2^31-1 octets in settings frame"));
                }
                self.settings.push(Setting::InitialWindowSize(val as i32))
            },
            0x5 => {
                if val < MIN_FRAME_SIZE || val > MAX_FRAME_SIZE {
                    return Err(Error::new(ErrorKind::Protocol, "Max frame size must be between 2^14 and 2^24-1 octets in settings frame"));
                }
                self.settings.push(Setting::MaxFrameSize(val))
            },
            0x6 => self.settings.push(Setting::MaxHeaderListSize(val)),
            _ => {}, // discard unknown setings
        }
        Ok(())
    }

    fn write_payload<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut buf = [0; 6];
        for setting in self.settings.iter() {
            let (id, val) = match *setting {
                Setting::HeaderTableSize(val) => (0x1, val),
                Setting::EnablePush(val) => (0x2, val as u32),
                Setting::MaxConcurrentStreams(val) => (0x3, val),
                Setting::InitialWindowSize(val) => (0x4, val as u32),
                Setting::MaxFrameSize(val) => (0x5, val),
                Setting::MaxHeaderListSize(val) => (0x6, val),
            };
            BigEndian::write_u16(&mut buf, id);
            BigEndian::write_u32(&mut buf[2..], val);
            try!(writer.write(&buf));
        }
        Ok(())
    }
}

impl Frame for SettingsFrame {
    fn header(&self) -> FrameHeader {
        let flags = if self.ack { FLAG_ACK } else { Flags::empty() };
        FrameHeader {
            payload_len: SETTING_LENGTH * self.settings.len(),
            frame_type: TYPE_SETTINGS,
            flags: flags,
            stream_id: StreamId(0),
        }
    }

    fn write<W: Write>(self, writer: &mut W) -> Result<()> {
        self.write_payload(writer)
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;
    use super::{Setting, SettingsFrame};
    use super::super::super::StreamId;
    use frame::{ReadFrame, FrameHeader, WriteFrame, FrameKind, Flags, TYPE_SETTINGS};
    use error::ErrorKind;

    fn single_setting_frame_header() -> FrameHeader {
        FrameHeader {
            payload_len: 6,
            frame_type: TYPE_SETTINGS,
            flags: Flags::empty(),
            stream_id: StreamId(0),
        }
    }

    #[test]
    fn test_empty_settings_frame() {
        let frame = SettingsFrame::default();
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 0, 4, 0, 0, 0, 0, 0]);
        let mut sl = &b[..];
        let res = match sl.read_frame(100).unwrap() {
            FrameKind::Settings(frame) => frame,
            _ => panic!("Wrong frame type")
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_ack_settings_frame() {
        let frame = SettingsFrame::ack();
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 0, 4, 1, 0, 0, 0, 0]);
        let mut sl = &b[..];
        if let FrameKind::Settings(res) = sl.read_frame(100).unwrap() {
            assert_eq!(frame, res);
            assert!(res.is_ack());
        } else {
            panic!("Wrong frame type")
        }
    }

    #[test]
    fn test_full_settings_frame() {
        let mut frame = SettingsFrame::default();
        frame.add_setting(Setting::HeaderTableSize(100));
        frame.add_setting(Setting::EnablePush(false));
        frame.add_setting(Setting::MaxConcurrentStreams(100));
        frame.add_setting(Setting::InitialWindowSize(100));
        frame.add_setting(Setting::MaxFrameSize(100000));
        frame.add_setting(Setting::MaxHeaderListSize(100));
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        let mut sl = &b[..];
        let res = match sl.read_frame(1000).unwrap() {
            FrameKind::Settings(frame) => frame,
            _ => panic!("Wrong frame type")
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_ack_and_payload_error() {
        let mut frame = SettingsFrame::ack();
        frame.add_setting(Setting::EnablePush(true));
        let mut b = Vec::new();
        b.write_frame(frame).unwrap();
        assert_eq!(b, [0, 0, 6, 4, 1, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1]);
        let mut sl = &b[..];
        assert_eq!(sl.read_frame(1000).unwrap_err().kind(), ErrorKind::FrameSize);
    }

    #[test]
    fn test_wrong_stream_id_error() {
        let mut b = &vec![0, 0, 0, 4, 0, 0, 0, 0, 100][..];
        assert_eq!(b.read_frame(1000).unwrap_err().kind(), ErrorKind::Protocol);
    }

    #[test]
    fn test_invalid_enable_push_frame_error () {
        // enable_push value > 1
        let payload = Cursor::new([0, 2, 0, 0, 0, 100]);
        assert_eq!(SettingsFrame::read(single_setting_frame_header(), payload).unwrap_err().kind(), ErrorKind::Protocol);
    }

    #[test]
    fn test_invalid_initial_window_size() {
        let payload = Cursor::new([0, 4, 129, 255, 255, 255]);
        assert_eq!(SettingsFrame::read(single_setting_frame_header(), payload).unwrap_err().kind(), ErrorKind::FlowControl);
    }

    #[test]
    fn test_invalid_max_frame_size() {
        let mut payload = Cursor::new([0, 5, 0, 0, 0, 10]);
        assert_eq!(SettingsFrame::read(single_setting_frame_header(), payload).unwrap_err().kind(), ErrorKind::Protocol);
        payload = Cursor::new([0, 5, 255, 255, 255, 255]);
        assert_eq!(SettingsFrame::read(single_setting_frame_header(), payload).unwrap_err().kind(), ErrorKind::Protocol);
    }
}
