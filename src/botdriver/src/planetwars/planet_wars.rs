extern crate serde_json;

use std::collections::HashMap;
use bot_runner::BotHandle;

use futures::{Future, Async, Poll};
use futures::future::{join_all, JoinAll};

use std::io;
use std::io::Read;
use std::fs::File;

use planetwars::protocol;
use planetwars::rules::*;
use planetwars::player::{PlayerHandle, Prompt};
use planetwars::logger::JSONLogger;

pub struct Match {
    state: PlanetWars,
    prompts: JoinAll<Vec<Prompt<PlayerHandle>>>,
    logger: JSONLogger,
}

impl Future for Match {
    // names of winners
    type Item = Vec<String>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let prompts = match self.prompts.poll() {
                Ok(Async::Ready(prompts)) => prompts,
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_) => panic!("this should never happen"),
            };
            self.execute_commands(&prompts);
            self.state.step();
            
            if self.state.is_finished() {
                // TODO: move this logic
                
                let alive = self.state.players.values().filter_map(|p| {
                    if p.alive {
                        Some(p.name.clone())
                    } else {
                        None
                    }
                }).collect();
                return Ok(Async::Ready(alive));
            } else {
                let handles = prompts.into_iter().map(|(_, handle)| handle).collect();
                let future = prompt_players(&self.state, handles);
                self.prompts = future;
            }
        }
    }
}

impl Match {
    // TODO: split this method a bit
    pub fn new(mut players: HashMap<String, BotHandle>, conf: PlanetWarsConf) -> Self {
        // construct player map
        let player_map: HashMap<usize, Player> = players.keys().enumerate()
            .map(|(num, name)| {
                let player = Player {
                    id: num,
                    name: name.clone(),
                    alive: true,
                };
                return (num, player);
            }).collect();

        // construct planet map
        let planets = conf.load_map(&player_map).into_iter()
            .map(|planet| {
                (planet.name.clone(), planet)
            }).collect();

        let handles = player_map.values().map(|player| {
            let handle = players.remove(&player.name).unwrap();
            return PlayerHandle::new(player.id, handle);
        }).collect();

        let mut state = PlanetWars {
            planets: planets,
            players: player_map,
            expeditions: Vec::new(),
            expedition_num: 0,
            turn_num: 0,
            max_turns: conf.max_turns,
        };

        // TODO: log state

        return Match {
            prompts: prompt_players(&state, handles),
            state: state,
            logger: JSONLogger::new("log.jsonl"),
        }
    }

    fn execute_commands(&mut self, commands: &Vec<(String, PlayerHandle)>) {
        for &(ref command, ref player) in commands {
            let r = serde_json::from_str::<protocol::Command>(command.as_str());
            if let Ok(cmd) = r {
                for mv in cmd.moves.iter() {
                    if self.move_valid(player.id(), mv) {
                        self.state.dispatch(mv);
                    }
                }
            }
        }
    }
    
    
    fn log_state(&mut self) {
        unimplemented!()
        // let state = self.state.repr();
        // self.log.log_json(&state)
        //     .expect("[PLANET_WARS] logging failed");
    }

    
    fn move_valid(&mut self, player_id: usize, m: &protocol::Move) -> bool {
        // TODO: this code sucks.
        // TODO: actually handle errors
        // MOZAIC should support soft errors first, of course.
        // Alternatively, a game implementation could be made responsible for
        // this. This would require more work, but also allow more flexibility.


        // check whether origin and target exist
        if !self.state.planets.contains_key(&m.origin) {
            return false;
        }
        if !self.state.planets.contains_key(&m.destination) {
            return false;
        }

        // check whether player owns origin and has enough ships there
        let origin = &self.state.planets[&m.origin];
        
        if origin.owner() != Some(player_id) {
            return false;
        }
        if origin.ship_count() < m.ship_count {
            return false;
        }

        true
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PlanetWarsConf {
    pub map_file: String,
    pub player_map: HashMap<String, String>,
    pub max_turns: u64,
}

impl PlanetWarsConf {
    fn load_map(&self, players: &HashMap<usize, Player>) -> Vec<Planet> {
        let map = self.read_map().expect("[PLANET_WARS] reading map failed");
        
        let player_translation: HashMap<&str, usize> = players.iter()
            .map(|(&id, player)| {
                (self.player_map.get(&player.name).unwrap().as_str(), id)
            }).collect();

        return map.planets.into_iter().map(|planet| {
            let mut fleets = Vec::new();
            if planet.ship_count > 0 {
                fleets.push(Fleet {
                    owner: planet.owner.and_then(|ref owner| {
                        player_translation.get(owner.as_str()).map(|&v| v)
                    }),
                    ship_count: planet.ship_count,
                });
            }
            return Planet {
                name: planet.name,
                x: planet.x,
                y: planet.y,
                fleets: fleets,
            };
        }).collect();
    }

    fn read_map(&self) -> io::Result<protocol::Map> {
        let mut file = File::open(&self.map_file)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        let map = serde_json::from_str(&buf)?;
        return Ok(map);
    }
}

// TODO: as this logic gets more complicated, it might be good to
// sparate this functionality into a purpose-specific struct.
fn prompt_players(state: &PlanetWars, handles: Vec<PlayerHandle>)
                  -> JoinAll<Vec<Prompt<PlayerHandle>>>
{
    let prompts = handles.into_iter().filter_map(|handle| {
        let player = &state.players[&handle.id()];
        
        if player.alive {
            let state = state.repr();
            let p = serde_json::to_string(&state)
                .expect("[PLANET_WARS] Serializing game state failed.");
            Some(handle.prompt(p))
        } else {
            None
        }
    }).collect();
    return join_all(prompts);
}
