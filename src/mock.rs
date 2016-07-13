use std::io::{Read, Write, Cursor, Result};
use std::rc::Rc;
use std::cell::RefCell;
use connection::Connection;

#[derive(Debug)]
pub struct MockStream {
    rx: Rc<RefCell<Cursor<Vec<u8>>>>,
    tx: Rc<RefCell<Cursor<Vec<u8>>>>,
}

impl MockStream {
    pub fn new() -> (MockStream, MockStream) {
        let rx = Rc::new(RefCell::new(Cursor::new(Vec::new())));
        let tx = Rc::new(RefCell::new(Cursor::new(Vec::new())));
        (MockStream {
            rx: rx.clone(),
            tx: tx.clone(),
        },
         MockStream {
            rx: tx.clone(),
            tx: rx.clone(),
        })
    }
}

impl Write for MockStream {
    fn write<'a>(&mut self, buf: &'a [u8]) -> Result<usize> {
        self.tx.borrow_mut().get_mut().write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.tx.borrow_mut().get_mut().flush()
    }
}

impl Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.rx.borrow_mut().read(buf)
    }
}

impl Connection for MockStream {}

#[cfg(test)]
mod test {
    use super::MockStream;
    use std::io::{Read, Write};

    #[test]
    fn test_mem_writer() {
        let (mut server, mut client) = MockStream::new();
        assert_eq!(server.write(&[0]).unwrap(), 1);
        assert_eq!(server.write(&[1, 2, 3]).unwrap(), 3);
        assert_eq!(server.write(&[4, 5, 6, 7]).unwrap(), 4);
        let b: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7];
        let mut buf = [0; 8];
        assert_eq!(client.read(&mut buf).unwrap(), 8);
        assert_eq!(buf, b);
    }

    #[test]
    fn test_mem_reader() {
        let (mut server, mut client) = MockStream::new();
        assert_eq!(server.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap(), 8);
        let mut buf = [];
        assert_eq!(client.read(&mut buf).unwrap(), 0);
        let mut buf = [0];
        assert_eq!(client.read(&mut buf).unwrap(), 1);
        let b: &[_] = &[0];
        assert_eq!(buf, b);
        let mut buf = [0; 4];
        assert_eq!(client.read(&mut buf).unwrap(), 4);
        let b: &[_] = &[1, 2, 3, 4];
        assert_eq!(buf, b);
        assert_eq!(server.write(&[0, 1, 2, 3, 4, 5, 6, 7]).unwrap(), 8);
        assert_eq!(client.read(&mut buf).unwrap(), 4);
        let b: &[_] = &[5, 6, 7, 0];
        assert_eq!(buf, b);
        assert_eq!(client.read(&mut buf).unwrap(), 4);
        let b: &[_] = &[1, 2, 3, 4];
        assert_eq!(buf, b);
    }
}
