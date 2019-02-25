use std::env;

extern crate sodiumoxide;
extern crate serde_json;
extern crate tokio;
extern crate futures;
extern crate mozaic;
extern crate rand;

// use mozaic::server::{Config as ServerConfig, Server};

use std::net::SocketAddr;
use tokio::prelude::Stream;
use futures::Async;
use futures::sync::mpsc;
use mozaic::network_capnp::{connect, connected, publish};
use mozaic::core_capnp::{initialize, actor_joined, greeting};
use mozaic::messaging::runtime::{Broker, BrokerHandle};
use mozaic::messaging::types::*;
use mozaic::messaging::reactor::*;
use mozaic::server::run_server;

use rand::Rng;

// Load the config and start the game.
fn main() {
    run(env::args().collect());
}


struct Welcomer {
    runtime_id: Uuid,
}

impl Welcomer {
    fn params<C: Ctx>(self) -> CoreParams<Self, C> {
        let mut params = CoreParams::new(self);
        params.handler(initialize::Owned, CtxHandler::new(Self::initialize));
        params.handler(actor_joined::Owned, CtxHandler::new(Self::welcome));
        return params;
    }

    fn initialize<C: Ctx>(
        &mut self,
        handle: &mut ReactorHandle<C>,
        _: initialize::Reader,
    ) -> Result<(), capnp::Error>
    {
        let link = WelcomerRuntimeLink {};
        handle.open_link(link.params(self.runtime_id.clone()));
        return Ok(());
    }

    fn welcome<C: Ctx>(
        &mut self,
        handle: &mut ReactorHandle<C>,
        r: actor_joined::Reader,
    ) -> Result<(), capnp::Error>
    {
        let id: Uuid = r.get_id()?.into();
        println!("welcoming {:?}", id);
        let link = WelcomerGreeterLink {};
        handle.open_link(link.params(id));
        return Ok(());
    }

}

struct WelcomerRuntimeLink {}

impl WelcomerRuntimeLink {
    fn params<C: Ctx>(self, foreign_uuid: Uuid) -> LinkParams<Self, C> {
        let mut params = LinkParams::new(foreign_uuid, self);
        params.external_handler(
            actor_joined::Owned,
            CtxHandler::new(Self::welcome),
        );

        return params;
    }

    fn welcome<C: Ctx>(
        &mut self,
        handle: &mut LinkHandle<C>,
        r: actor_joined::Reader,
    ) -> Result<(), capnp::Error>
    {
        let id: Uuid = r.get_id()?.into();
        handle.send_internal(actor_joined::Owned, |b| {
            let joined: actor_joined::Builder = b.init_as();
            set_uuid(joined.init_id(), &id);
        });
        return Ok(());
    }
}

struct WelcomerGreeterLink {}

impl WelcomerGreeterLink {
    fn params<C: Ctx>(self, foreign_uuid: Uuid) -> LinkParams<Self, C> {
        let mut params = LinkParams::new(foreign_uuid, self);
        params.external_handler(
            greeting::Owned,
            CtxHandler::new(Self::recv_greeting),
        );

        return params;
    }

    fn recv_greeting<C: Ctx>(
        &mut self,
        handle: &mut LinkHandle<C>,
        greeting: greeting::Reader,
    ) -> Result<(), capnp::Error>
    {
        let message = greeting.get_message()?;
        println!("got greeting: {:?}", message);
        handle.close_link();
        return Ok(());
    }
}

pub fn run(_args : Vec<String>) {

    let addr = "127.0.0.1:9142".parse::<SocketAddr>().unwrap();

    run_server(addr, |runtime_id| {
        let w = Welcomer { runtime_id };
        return w.params();
    });
}
