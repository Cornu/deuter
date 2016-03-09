extern crate byteorder;

#[cfg(test)]
mod mock;

mod error;
mod connection;
mod frame;
mod client;

use frame::settings::{Setting, SettingsFrame};

#[derive(PartialEq)]
pub struct StreamId(u32);

impl PartialEq<u32> for StreamId {
    fn eq(&self, other: &u32) -> bool { self.0 == *other }
}

pub struct Settings {
    pub header_table_size: u32,
    pub enable_push: bool,
    pub max_concurrent_streams: Option<u32>,
    pub initial_window_size: i32,
    pub max_frame_size: u32,
    pub max_header_list_size: Option<u32>
}

impl Settings {
    fn update(&mut self, frame: SettingsFrame) {
        for setting in frame.settings() {
            match setting {
                Setting::HeaderTableSize(val) => self.header_table_size = val,
                Setting::EnablePush(val) => self.enable_push = val,
                Setting::MaxConcurrentStreams(val) => self.max_concurrent_streams = Some(val),
                // TODO update streams with new window size
                Setting::InitialWindowSize(val) => self.initial_window_size = val,
                Setting::MaxFrameSize(val) => self.max_frame_size = val,
                Setting::MaxHeaderListSize(val) => self.max_header_list_size = Some(val),
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            header_table_size: 4096,
            enable_push: true,
            max_concurrent_streams: None,
            initial_window_size: 65536,
            max_frame_size: 16384,
            max_header_list_size: None,
        }
    }
}

pub struct WindowSize(i32);

impl WindowSize {
    fn available(&self) -> usize {
        if self.0.is_negative() {
            return 0
        }
        self.0 as usize
    }

    fn set(&mut self, n: i32) {
        self.0 = n;
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize(65535)
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};

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
}
