use bytes::BytesMut;
use futures::{Future, Poll, Async, Stream};
use futures::sink::{Sink, Send};
use futures::sync::mpsc::UnboundedSender;
use prost::Message;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio;
use tokio::net::{Incoming, TcpListener, TcpStream};

use protobuf_codec::{MessageStream, ProtobufTransport};
use protocol;
use super::router::{RoutingTable, RoutingMessage};


pub struct Listener {
    incoming: Incoming,
    routing_table: Arc<Mutex<RoutingTable>>,
}

impl Listener {
    pub fn new(addr: &SocketAddr, routing_table: Arc<Mutex<RoutingTable>>)
               -> io::Result<Self>
    {
        TcpListener::bind(addr).map(|tcp_listener| {
            Listener {
                routing_table,
                incoming: tcp_listener.incoming(),
            }
        })
    }

    fn handle_connections(&mut self) -> Poll<(), io::Error> {
        while let Some(raw_stream) = try_ready!(self.incoming.poll()) {
            let handler = ConnectionHandler::new(
                self.routing_table.clone(),
                raw_stream
            );
            tokio::spawn(handler);
        }
        return Ok(Async::Ready(()));
    }
}

impl Future for Listener {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        match self.handle_connections() {
            Ok(async) => return Ok(async),
            // TODO: gracefully handle this
            Err(e) => panic!("error: {}", e),
        }
    }
}


struct Waiting {
    transport: ProtobufTransport<TcpStream>,
    routing_table: Arc<Mutex<RoutingTable>>,
}

enum Action {
    Accept {
        handle: UnboundedSender<RoutingMessage>,
    },
    Refuse {
        reason: String,
    }
}

impl Waiting {
    fn poll(&mut self) -> Poll<Action, io::Error>
    {
        let bytes = match try_ready!(self.transport.poll()) {
            None => bail!(io::ErrorKind::ConnectionAborted),
            Some(bytes) => bytes.freeze(),
        };

        let request = try!(protocol::ConnectionRequest::decode(bytes));

        let mut table = self.routing_table.lock().unwrap(); 
        let action = match table.get(&request.token) {
            None => Action::Refuse { reason: "invalid token".to_string() },
            Some(handle) => Action::Accept { handle },
        };
        return Ok(Async::Ready(action));
    }

    fn step(self, action: Action) -> Result<HandlerState, io::Error> {
        match action {
            Action::Accept { handle } => {
                let response = protocol::ConnectionResponse {
                    response: Some(
                        protocol::connection_response::Response::Success(
                            protocol::ConnectionSuccess {}
                        )
                    )
                };
                let mut buf = BytesMut::new();
                try!(response.encode(&mut buf));

                let accepting = Accepting {
                    send: self.transport.send(buf),
                    handle,
                };
                return Ok(HandlerState::Accepting(accepting));
            },
            Action::Refuse { reason } => {
                let response = protocol::ConnectionResponse {
                    response: Some(
                        protocol::connection_response::Response::Error(
                            protocol::ConnectionError {
                                message: reason,
                            }
                        )
                    )
                };
                let mut buf = BytesMut::new();
                try!(response.encode(&mut buf));

                let refusing = Refusing {
                    send: self.transport.send(buf),
                };
                return Ok(HandlerState::Refusing(refusing));
            }
        }
    }
}


struct Accepting {
    send: Send<ProtobufTransport<TcpStream>>,
    handle: UnboundedSender<RoutingMessage>,
}

impl Accepting {
    fn poll(&mut self) -> Poll<ProtobufTransport<TcpStream>, io::Error> {
        self.send.poll()
    }

    fn step(self, transport: ProtobufTransport<TcpStream>) -> HandlerState {
         self.handle.unbounded_send(RoutingMessage::Connecting {
            stream: MessageStream::new(transport)
        }).unwrap();
        return HandlerState::Done;
    }
}

struct Refusing {
    send: Send<ProtobufTransport<TcpStream>>,
}

impl Refusing {
    fn poll(&mut self) -> Poll<(), io::Error> {
        try_ready!(self.send.poll());
        return Ok(Async::Ready(()));
    }

    fn step(self) -> HandlerState {
        return HandlerState::Done;
    }
}

enum HandlerState {
    Waiting(Waiting),
    Accepting(Accepting),
    Refusing(Refusing),
    Done,
}

pub struct ConnectionHandler {
    state: HandlerState,
}

impl ConnectionHandler {
    pub fn new(routing_table: Arc<Mutex<RoutingTable>>,
               stream: TcpStream) -> Self
    {
        let transport = ProtobufTransport::new(stream);
        ConnectionHandler {
            state: HandlerState::Waiting(Waiting {
                transport,
                routing_table,
            }),
        }
    }
}

impl ConnectionHandler {
    // TODO: can we get rid of this boilerplate?
    fn step(&mut self) -> Poll<(), io::Error> {
        loop {
            let state = mem::replace(&mut self.state, HandlerState::Done);
            match state {
                HandlerState::Waiting(mut waiting) => {
                    match try!(waiting.poll()) {
                        Async::NotReady => {
                            self.state = HandlerState::Waiting(waiting);
                            return Ok(Async::NotReady);
                        }
                        Async::Ready(action) => {
                            self.state = try!(waiting.step(action));
                        }
                    }
                }
                HandlerState::Accepting(mut accepting) => {
                    match try!(accepting.poll()) {
                        Async::NotReady => {
                            self.state = HandlerState::Accepting(accepting);
                            return Ok(Async::NotReady);
                        }
                        Async::Ready(transport) => {
                            self.state = accepting.step(transport);
                        }
                    }
                }
                HandlerState::Refusing(mut refusing) => {
                    match try!(refusing.poll()) {
                        Async::NotReady => {
                            self.state = HandlerState::Refusing(refusing);
                            return Ok(Async::NotReady);
                        }
                        Async::Ready(()) => {
                            self.state = refusing.step();
                        }
                    }
                }
                HandlerState::Done => {
                    return Ok(Async::Ready(()));
                }
            }
        }
    }
}

impl Future for ConnectionHandler {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        match self.step() {
            // TODO: handle this case gracefully
            Err(err) => panic!("error: {}", err),
            Ok(poll) => Ok(poll),
        }
    }
}