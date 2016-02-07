use byteorder::{ByteOrder, BigEndian};
use frame::{Frame, Flags};
use error::{Error, ErrorKind, Result};

const MAX_FLOW_CONTROL_WINDOW_SIZE : u32 = ::std::i32::MAX as u32;
const MIN_FRAME_SIZE : u32 = 16384;
const MAX_FRAME_SIZE : u32 = 16777215;

pub const FLAG_ACK : Flags = 0x1;

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
    ack: bool
}

impl SettingsFrame {
    pub fn from_raw(stream_id: u32, flags: u8, payload: &[u8]) -> Result<SettingsFrame> {
        if stream_id != 0 {
            return Err(Error::new(ErrorKind::Protocol, "The stream identifier for a settings frame must be zero"));
        }
        let mut frame: Self = Default::default();
        if flags & FLAG_ACK != 0 {
            frame.ack = true;
            if !payload.is_empty() {
                return Err(Error::new(ErrorKind::FrameSize, "Settings Frame with Ack Flag must be empty"));
            }
        }
        if !payload.is_empty() {
            frame.settings = try!(parse_payload(payload));
        }
        Ok(frame)
    }

    pub fn ack() -> SettingsFrame {
        SettingsFrame {
            settings: Vec::new(),
            ack: true,
        }
    }

    pub fn add_setting(&mut self, set: Setting) {
        self.settings.push(set);
    }

    pub fn is_ack(&self) -> bool {
        self.ack
    }
}

fn parse_payload(payload: &[u8]) -> Result<Vec<Setting>> {
    if payload.len() % 6 != 0 {
        return Err(Error::new(ErrorKind::FrameSize, "Settings Frame payload length must be multiple of 6"));
    }
    let n = payload.len() / 6;
    let mut settings = Vec::with_capacity(n);
    for p in 0..n {
        let id = BigEndian::read_u16(&payload[p*6..]);
        let val = BigEndian::read_u32(&payload[p*6+2..]);
        // parse according to rfc 6.5.2
        match id {
            0x1 => settings.push(Setting::HeaderTableSize(val)),
            0x2 => match val {
                0 => settings.push(Setting::EnablePush(false)),
                1 => settings.push(Setting::EnablePush(true)),
                _ => return Err(Error::new(ErrorKind::Protocol, "Invalid Value for enable push setting in settings frame")),
            },
            0x3 => settings.push(Setting::MaxConcurrentStreams(val)),
            0x4 => {
                if val > MAX_FLOW_CONTROL_WINDOW_SIZE {
                    return Err(Error::new(ErrorKind::FlowControl, "Initial window size must be lower than 2^31-1 octets in settings frame"));
                }
                settings.push(Setting::InitialWindowSize(val as i32))
            },
            0x5 => {
                if val < MIN_FRAME_SIZE || val > MAX_FRAME_SIZE {
                    return Err(Error::new(ErrorKind::Protocol, "Max frame size must be between 2^14 and 2^24-1 octets in settings frame"));
                }
                settings.push(Setting::MaxFrameSize(val))
            },
            0x6 => settings.push(Setting::MaxHeaderListSize(val)),
            _ => continue
        }
    }
    Ok(settings)
}

impl Frame for SettingsFrame {
    #[inline]
    fn payload_len(&self) -> usize {
        // each settings consists of an 2 byte identifier and 4 byte value
        6 * self.settings.len()
    }

    #[inline]
    fn frame_type(&self) -> u8 {
        0x4
    }

    #[inline]
    fn flags(&self) -> u8 {
        self.ack as u8
    }

    #[inline]
    fn stream_id(&self) -> u32 {
        // stream identifier for a settings frame must be zero
        0
    }
}

impl Into<Vec<u8>> for SettingsFrame {
    fn into(self) -> Vec<u8> {
        let mut buf = vec![0; self.payload_len()];
        for (i, setting) in self.settings.iter().enumerate() {
            let (id, val) = match *setting {
                Setting::HeaderTableSize(val) => (0x1, val),
                Setting::EnablePush(val) => (0x2, val as u32),
                Setting::MaxConcurrentStreams(val) => (0x3, val),
                Setting::InitialWindowSize(val) => (0x4, val as u32),
                Setting::MaxFrameSize(val) => (0x5, val),
                Setting::MaxHeaderListSize(val) => (0x6, val),
            };
            BigEndian::write_u16(&mut buf[i*6..], id);
            BigEndian::write_u32(&mut buf[i*6+2..], val);
        }
        buf
    }
}

#[cfg(test)]
mod test {
    use super::{Setting, SettingsFrame};
    use frame::{ReadFrame, WriteFrame, FrameType};
    use error::ErrorKind;

    #[test]
    fn test_empty_settings_frame() {
        let frame = SettingsFrame::default();
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
        assert_eq!(b, [0, 0, 0, 4, 0, 0, 0, 0, 0]);
        let mut sl = &b[..];
        let res = match sl.read_frame(100).unwrap() {
            FrameType::Settings(frame) => frame,
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
        if let FrameType::Settings(res) = sl.read_frame(100).unwrap() {
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
            FrameType::Settings(frame) => frame,
            _ => panic!("Wrong frame type")
        };
        assert_eq!(frame, res);
    }

    #[test]
    fn test_ack_and_payload_error() {
        let mut frame = SettingsFrame::ack();
        frame.add_setting(Setting::EnablePush(true));
        let mut b = Vec::new();
        b.write_frame(frame.clone()).unwrap();
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
        let payload = [0, 2, 0, 0, 0, 100];
        assert_eq!(SettingsFrame::from_raw(0, 0, &payload).unwrap_err().kind(), ErrorKind::Protocol);
    }

    #[test]
    fn test_invalid_initial_window_size() {
        let payload = [0, 4, 129, 255, 255, 255];
        assert_eq!(SettingsFrame::from_raw(0, 0, &payload).unwrap_err().kind(), ErrorKind::FlowControl);
    }

    #[test]
    fn test_invalid_max_frame_size() {
        let mut payload = [0, 5, 0, 0, 0, 10];
        assert_eq!(SettingsFrame::from_raw(0, 0, &payload).unwrap_err().kind(), ErrorKind::Protocol);
        payload = [0, 5, 255, 255, 255, 255];
        assert_eq!(SettingsFrame::from_raw(0, 0, &payload).unwrap_err().kind(), ErrorKind::Protocol);
    }
}
