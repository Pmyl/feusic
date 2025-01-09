use std::{error::Error, time::Duration};

use egui::Ui;

use crate::core::{feusic::loader::MusicLoader, player::FeusicPlayerController};

pub(super) fn render<M: MusicLoader>(
    ui: &Ui,
    player: &FeusicPlayerController<M>,
) -> Result<(), Box<dyn Error>> {
    egui::Window::new("Hello world")
        .open(&mut true)
        .show(&ui.ctx(), |ui| {
            ui.label(format!("Duration: {}", player.music_duration().as_secs()));
            ui.label(format!("Position: {}", player.music_position().as_secs()));
            ui.separator();
            ui.label("Controls");
            ui.separator();

            if ui.button("Next").clicked() {
                player.next();
            }

            if ui.button("Crossfade").clicked() {
                player.crossfade(Duration::from_millis(1000));
            }

            if ui.button("Pause").clicked() {
                player.pause();
            }

            if ui.button("Resume").clicked() {
                player.resume();
            }

            if ui.button("Stop").clicked() {
                player.stop();
            }
        });

    Ok(())
}
