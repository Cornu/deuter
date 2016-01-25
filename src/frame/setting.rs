use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use frame::{Frame, FrameType};
use error::ConnectionError;

/// Settings Parameter according to rfc 6.5.2
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    HeaderTableSize(u32),
    EnablePush(u32),
    MaxConcurrentStreams(u32),
    InitialWindowSize(u32),
    MaxFrameSize(u32),
    MaxHeaderListSize(u32)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SettingsFrame {
    settings: Vec<Setting>,
    ack: bool
}

impl SettingsFrame {
    pub fn from_raw(stream_id: u32, flags: u8, payload: &[u8]) -> Result<SettingsFrame, ConnectionError> {
        if stream_id != 0 {
            return Err(ConnectionError::Protocol)
        }
        let mut frame: Self = Default::default();
        if flags & 0x1 != 0 {
            frame.set_ack(true);
        }
        if !payload.is_empty() {
            frame.settings = try!(parse_payload(payload));
        }
        Ok(frame)
    }

    pub fn is_ack(&self) -> bool {
        self.ack
    }

    pub fn set_ack(&mut self, ack: bool) {
        self.ack = ack;
    }
}

fn parse_payload(payload: &[u8]) -> Result<Vec<Setting>, ConnectionError> {
    if payload.len() % 6 != 0 {
        return Err(ConnectionError::FrameSize);
    }
    let n = payload.len() / 6;
    let mut settings = Vec::with_capacity(n);
    for p in 0..n {
        let id = BigEndian::read_u16(&payload[p..]);
        let val = BigEndian::read_u32(&payload[p+2..]);
        // parse according to rfc 6.5.2
        match id {
            0x1 => settings.push(Setting::HeaderTableSize(val)),
            0x2 => settings.push(Setting::EnablePush(val)),
            0x3 => settings.push(Setting::MaxConcurrentStreams(val)),
            0x4 => settings.push(Setting::InitialWindowSize(val)),
            0x5 => settings.push(Setting::MaxFrameSize(val)),
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
        // TODO add flags
        0
    }

    #[inline]
    fn stream_id(&self) -> u32 {
        // stream identifier for a settings frame must be zero
        0
    }
}

#[cfg(test)]
mod test {
    use super::SettingsFrame;
    use frame::{ReadFrame, WriteFrame, FrameType};

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
}
