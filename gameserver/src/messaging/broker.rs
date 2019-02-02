use std::collections::HashMap;
use futures::sync::mpsc;
use futures::{Future, Stream, Async, Poll};

use super::reactor::{Uuid, Message, ReactorSpawner, Reactor};

enum BrokerCmd {
    Send(Message),
    Spawn(Box<ReactorSpawner>),
    Unregister(Uuid),
}

pub struct BrokerHandle {
    inner: mpsc::UnboundedSender<BrokerCmd>,
}

impl BrokerHandle {
    fn send_cmd(&mut self, cmd: BrokerCmd) {
        self.inner.unbounded_send(cmd)
            .expect("broker channel dropped");
    }

    pub fn send(&mut self, message: Message) {
        self.send_cmd(BrokerCmd::Send(message));
    }

    pub fn spawn(&mut self, spawner: Box<ReactorSpawner>) {
        self.send_cmd(BrokerCmd::Spawn(spawner));
    }

    pub fn unregister(&mut self, uuid: Uuid) {
        self.send_cmd(BrokerCmd::Unregister(uuid));
    }
}

pub struct Broker {
    recv: mpsc::UnboundedReceiver<BrokerCmd>,
    snd: mpsc::UnboundedSender<BrokerCmd>,
    actors: HashMap<Uuid, mpsc::UnboundedSender<Message>>,
}

impl Broker {
    pub fn new() -> Self {
        let (snd, recv) = mpsc::unbounded();

        Broker {
            recv,
            snd,
            actors: HashMap::new(),
        }
    }

    pub fn get_handle(&self) -> BrokerHandle {
        BrokerHandle {
            inner: self.snd.clone(),
        }
    }

    pub fn add_actor(&mut self, uuid: Uuid, snd: mpsc::UnboundedSender<Message>)
    {
        self.actors.insert(uuid, snd);
    }

    pub fn spawn(&mut self, spawner: Box<ReactorSpawner>) {
        let uuid = spawner.reactor_uuid().clone();
        let reactor_handle = spawner.spawn_reactor(self.get_handle());
        self.actors.insert(uuid, reactor_handle);
    }

    pub fn unregister(&mut self, uuid: &Uuid) {
        self.actors.remove(uuid);
    }

    fn route_messages(&mut self) -> Poll<(), ()> {
        loop {
            // unwrapping is fine because we hold a handle
            match try_ready!(self.recv.poll()).unwrap() {
                BrokerCmd::Send(msg) => {
                    self.route_message(msg).unwrap();
                }
                BrokerCmd::Spawn(spawner) => {
                    self.spawn(spawner);
                }
                BrokerCmd::Unregister(uuid) => {
                    self.unregister(&uuid);
                }
            }
        }
    }

    fn route_message(&mut self, message: Message)
        -> Result<(), capnp::Error>
    {
        let chan = {
            let msg = message.reader()?;
            let receiver_uuid = msg.get_receiver()?.into();
            match self.actors.get_mut(&receiver_uuid) {
                Some(chan) => chan,
                None => {
                    eprintln!("unknown reciever: {:?}", receiver_uuid);
                    return Ok(());
                }
            }
        };
        chan.unbounded_send(message).unwrap();
        return Ok(());
    }
}

impl Future for Broker {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        try!(self.route_messages());

        if self.actors.is_empty() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}