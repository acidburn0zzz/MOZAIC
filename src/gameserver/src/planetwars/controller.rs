use std::collections::HashSet;

use futures::{Future, Async, Poll, Stream};
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};

use client_controller::{ClientMessage, Message};
use planetwars::config::Config;
use planetwars::step_lock::StepLock;
use planetwars::lock::Lock;
use planetwars::game_controller::GameController;
use planetwars::pw_controller::PwController;

use slog;

/// The controller forms the bridge between game rules and clients.
/// It is responsible for communications, the control flow, and logging.
pub struct Controller<L: Lock<GameController>>  {
    lock: L,
    client_msgs: UnboundedReceiver<ClientMessage>,
    logger: slog::Logger,
}


#[derive(Clone)]
pub struct Client {
    pub id: usize,
    pub player_name: String,
    pub handle: UnboundedSender<String>,
}

impl Client {
    pub fn send_msg(&mut self, msg: String) {
        // unbounded channels don't fail
        self.handle.unbounded_send(msg).unwrap();
    }
}

impl<L> Controller<L> 
    where L: Lock<GameController>
{
    // TODO: this method does both controller initialization and game staritng.
    // It would be nice to split these.
    pub fn new(clients: Vec<Client>,
               client_msgs: UnboundedReceiver<ClientMessage>,
               conf: Config, logger: slog::Logger,)
               -> Controller<L>
    {
        let mut client_ids = HashSet::new();
        client_ids.extend(clients.iter().map(|c| c.id));
        Controller {
            lock: Lock::new(GameController::new(conf, clients, logger.clone()), client_ids),
            client_msgs,
            logger
        }
    }

    /// Handle an incoming message.
    fn handle_message(&mut self, client_id: usize, msg: Message) {
        match msg {
            Message::Data(msg) => {
                // TODO: maybe it would be better to log this in the
                // client_controller.
                info!(self.logger, "message received";
                    "client_id" => client_id,
                    "content" => &msg,
                );
                self.lock.attach_command(client_id, msg);
            },
            Message::Disconnected => {
                // TODO: should a reason be included here?
                // It might be more useful to have the client controller log
                // disconnect reasons.
                info!(self.logger, "client disconnected";
                    "client_id" => client_id
                );
                self.lock.disconnect(client_id);
            }
            Message::Connected => {
                info!(self.logger, "client connected";
                    "client_id" => client_id
                );
                self.lock.connect(client_id);
            }
        }
    }
}

impl<L> Future for Controller<L>
    where L: Lock<GameController>
{
    type Item = Vec<usize>;
    type Error = ();

    fn poll(&mut self) -> Poll<Vec<usize>, ()> {
        loop {
            self.lock.act();
            let msg = try_ready!(self.client_msgs.poll()).unwrap();
            self.handle_message(msg.client_id, msg.message);

            while self.lock.is_ready() {
                if let Some(result) = self.lock.do_step() {
                    println!("Winner: {:?}", result);
                    return Ok(Async::Ready(result));
                }
            }
        }
    }
}
