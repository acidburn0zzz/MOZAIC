mod game;
mod bot_runner;
mod games;
mod match_runner;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate rand;

use std::error::Error;
use std::io::{Write, Read};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::fs::File;

use game::*;
use bot_runner::*;
use match_runner::*;

//use games::PlanetWars as Rules;

// Load the config and start the game.
fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 1 {
        let msg = format!("Expected 1 argument (config file). {} given.", args.len() - 1).to_owned();
        println!("{}", msg);
        std::process::exit(1)
    }

    let game_config: MatchConfig = match parse_config(Path::new(&args[1])) {
        Ok(config) => config,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1)
        }
    };


    // let mut runner = MatchRunner::<PlanetWars>::init(game_config);
    // runner.run();
}

// Parse a config passed to the program as an command-line argument.
// Return the parsed config.
pub fn parse_config(path: &Path) -> Result<MatchConfig, Box<Error>> {
    println!("Opening config {}", path.to_str().unwrap());
    let mut file = File::open(path)?;

    println!("Reading contents");
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    println!("Parsing config");
    let config: MatchConfig = serde_json::from_str(&contents)?;

    println!("Config parsed succesfully");
    Ok(config)
}

// Let the game play a step.
// Fetches all player responses to current state.
// Let's the game calculate next state.
// Return next state for next step (if game is not over yet).
// fn step<G: Game>(mut bots: &mut BotHandles, player_input: &PlayerInput, game: &mut G) -> GameStatus {
//     println!("Input:\n{}", pp(player_input));
//     let po = fetch_player_outputs(&player_input, &mut bots);
//     match po {
//         Ok(po) => {
//             println!("Output:\n{}", pp(&po));
//             return game.step(&po);
//         },
//         Err(e) => {
//             println!("{}", e);
//             std::process::exit(1)
//         }
//     }
// }

// Finnish up a game.
// Clean up any remaining botprocesses.
// fn finnish(bots: &mut BotHandles, outcome: Outcome) {
//     // Kill bot-processes
//     for (player, bot) in bots.iter_mut() {
//         bot.kill().expect(&format!("Unable to kill {}", player));
//     }
//     println!("Done with: {:#?}", outcome);
// }

// Fetch the responses from all players.
//
// TODO: Handle failure of fetching player response by propagating an empty response
//       This allows the gamerules to handle the problem.
// fn fetch_player_outputs(input: &PlayerInput, bots: &mut BotHandles) -> Result<PlayerOutput, Box<Error>> {
//     let mut pos = PlayerOutput::new();
//     for (player, info) in input.iter() {
//         println!("reading {}", player);
//         let mut bot = bots.get_mut(player)
//                           .expect(&format!("Response required for {} but no process found", player));
//         let po = match fetch_player_output(info, bot) {
//             Ok(po) => po,
//             Err(e) => return Err(e)
//         };
//         pos.insert(player.clone(), po);
//     }
//     Ok(pos)
// }

// Fetch the response for a particular player.
// Do this by passing his information, and (blockingly) wait for a response.
// fn fetch_player_output(info: &GameInfo, bot: &mut BotHandle) -> Result<PlayerCommand, Box<Error>> {
//     let msg_in = format!("Stdin not found for {:?}", bot);
//     let msg_out = format!("Stdout not found for {:?}", bot);

//     let bot_in = bot.stdin.as_mut().expect(&msg_in);
//     let bot_out = bot.stdout.as_mut().expect(&msg_out);
//     let mut bot_out = BufReader::new(bot_out);

//     bot_in.write_fmt(format_args!("{}\n", info))?;
//     bot_in.flush()?;
    
//     // TODO: Set overflow limit
//     let mut response = String::new();
//     bot_out.read_line(&mut response).expect("Invalid UTF-8 found");

//     Ok(response)
// }

// // Pretty print thing
// fn pp(hm: &HashMap<String, String>) -> String {
//     let mut pp = String::new();
//     for (key, value) in hm {
//         pp.push_str(&format!("# Value for {}:\n", key));
//         pp.push_str(&format!("{}\n", value));
//     }
//     pp
// }
