use std::{error::Error, time::Duration};

use imgui::{Condition, Ui};

use crate::core::{feusic::loader::MusicLoader, player::FeusicPlayerController};

pub(super) fn render<M: MusicLoader>(
    ui: &Ui,
    player: &FeusicPlayerController<M>,
) -> Result<(), Box<dyn Error>> {
    ui.window("Hello world")
        .size([300.0, 100.0], Condition::FirstUseEver)
        .resizable(false)
        .build(|| {
            ui.text(format!("Duration: {}", player.music_duration().as_secs()));
            ui.text(format!("Position: {}", player.music_position().as_secs()));
            ui.separator();
            ui.text("Controls");
            ui.separator();

            if ui.button("Next") {
                player.next();
            }

            if ui.button("Crossfade") {
                player.crossfade(Duration::from_millis(1000));
            }

            if ui.button("Pause") {
                player.pause();
            }

            if ui.button("Resume") {
                player.resume();
            }

            if ui.button("Stop") {
                player.stop();
            }
        });

    Ok(())
}
