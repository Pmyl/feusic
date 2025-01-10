use std::{error::Error, io::Write};

use crate::core::{feusic::loader::MusicLoader, player::FeusicPlayerController};

#[allow(unused)]
pub fn run_ui<M: MusicLoader>(player: FeusicPlayerController<M>) -> Result<(), Box<dyn Error>> {
    loop {
        println!("Commands: pause, resume, stop, loop, crossfade, next, exit");
        std::io::stdout().flush()?;

        let mut command = String::new();
        std::io::stdin().read_line(&mut command)?;
        let mut commands = command.trim().split(" ").collect::<Vec<_>>();
        let command = commands.remove(0);

        println!("Command received: {}", command);

        match command {
            "pause" => player.pause(),
            "resume" => player.resume(),
            "stop" => player.stop(),
            "next" => player.next(),
            "crossfade" => player.crossfade(std::time::Duration::from_secs(1)),
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }

    Ok(())
}
