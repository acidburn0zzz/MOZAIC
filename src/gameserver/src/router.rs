use std::collections::HashMap;
use futures::{Future, Poll, Stream};
use futures::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::net::TcpStream;
use client_controller::Command as ClientControllerCommand;

pub struct ConnectionRequest {
    pub stream: TcpStream,
    pub token: Vec<u8>,
}

pub enum RouterCommand {
    Connect(ConnectionRequest),
}

pub struct Router {
    connections: HashMap<Vec<u8>, UnboundedSender<ClientControllerCommand>>,
    ctrl_chan: UnboundedReceiver<RouterCommand>,
}

impl Router {
    fn handle_command(&mut self, cmd: RouterCommand) {
        match cmd {
            RouterCommand::Connect(request) => {
                let connection = self.connections.get(&request.token);
                if let Some(handle) = connection {
                    let cmd = ClientControllerCommand::Connect(request.stream);
                    handle.unbounded_send(cmd).unwrap();
                }
            },
        }
    }
}

impl Future for Router {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        loop {
            let cmd = try_ready!(self.ctrl_chan.poll()).unwrap();
            self.handle_command(cmd);
        }
    }
}