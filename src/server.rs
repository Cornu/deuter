use mio::{Handler, Token, EventLoop, EventSet, PollOpt};
use mio::tcp::{TcpListener};
use mio::util::Slab;
use std::net::SocketAddr;
use connection::Connection;
use error::{Error, ErrorKind, Result};

const SERVER : Token = Token(0);

struct Server {
    listener: TcpListener,
    connections: Slab<Connection>,
}

impl Server {
    fn new(listener: TcpListener) -> Self {
        let slab = Slab::new_starting_at(Token(1), 1024);
        Server {
            listener: listener,
            connections: slab,
        }
    }

    pub fn run(addr: SocketAddr) -> Result<()> {
        let listener = try!(TcpListener::bind(&addr));
        let mut event_loop = try!(EventLoop::new());
        try!(event_loop.register(&listener, SERVER, EventSet::readable(), PollOpt::edge()));
        let mut server = Self::new(listener);
        event_loop.run(&mut server);
        Ok(())
    }

    fn accept_new(&mut self, event_loop: &mut EventLoop<Server>) {
        match self.listener.accept() {
            Ok(Some((socket, addr))) => {
                info!("New Connection from {}", addr);
                let token = self.connections
                            .insert_with(|token| Connection::new(socket, token))
                            .unwrap();
                event_loop.register(
                            &self.connections[token].socket,
                            token,
                            EventSet::readable(), // TODO hup?
                            PollOpt::edge()).unwrap();
            }
            Ok(None) => {}
            Err(e) => {
                // TODO handle
                event_loop.shutdown();
            }
        }
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Server>, token: Token, events: EventSet) {
        match token {
            SERVER => self.accept_new(event_loop),
            _ => {
                if events.is_readable() { self.connections[token].read() }
                if events.is_writable() { self.connections[token].write() }
                if events.is_hup() {}
                if events.is_error() {}
                if self.connections[token].is_closed() {
                    event_loop.deregister(&self.connections[token].socket);
                    let _ = self.connections.remove(token);
                }
            }
        }
    }

    fn timeout(&mut self, event_loop: &mut EventLoop<Server>, timeout: Self::Timeout) {
    }
}

#[cfg(test)]
mod test {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpStream};
    extern crate env_logger;

    const HOST: &'static str = "127.0.0.1:60254";

    fn start_server() {
        use std::thread;
        use std::time::Duration;
        use std::sync::{Once, ONCE_INIT};

        static INIT: Once = ONCE_INIT;

        INIT.call_once(|| {
            thread::spawn(|| {
            info!("running server");
                super::Server::run(HOST.parse().unwrap()).unwrap();
            });
            thread::sleep(Duration::from_millis(1000));
        });
        println!("running");
    }

    #[test]
    fn test_server() {
        let _ = env_logger::init();
        start_server();

        let mut sock = BufReader::new(TcpStream::connect(HOST).unwrap());
        let mut recv = String::new();

        sock.get_mut().write_all(b"hello world\n").unwrap();

        assert_eq!(recv, "hello world\n");

        recv.clear();

        sock.get_mut().write_all(b"this is a line\n").unwrap();

        assert_eq!(recv, "this is a line\n")
    }
}

