use mio::{EventLoop, EventSet, Token};
use mio::tcp::TcpStream;

enum State {
    Preface,
    Settings,
    Closed,
}

pub struct Connection {
    pub socket: TcpStream,
    token: Token,
    state: State,
}

impl Connection {
    pub fn new(socket: TcpStream, token: Token) -> Connection {
        Connection {
            socket: socket,
            token: token,
            state: State::Preface,
        }
    }

    pub fn read(&self) {
        match self.state {
            State::Preface => self.read_preface(),
            State::Settings => self.read_settings(),
            _ => {}
        }
    }

    pub fn write(&self) {}

    pub fn is_closed(&self) -> bool {
        match self.state {
            State::Closed => true,
            _ => false,
        }
    }

    fn read_preface(&self) {}
    fn read_settings(&self) {}
}
