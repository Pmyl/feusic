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
            let duration = player.music_duration();
            let position = player.music_position();

            let mut relative_position = position.as_millis() as f32 / duration.as_millis() as f32;

            ui.add(egui::widgets::Slider::new(&mut relative_position, 0.0..=1.0).show_value(false));

            ui.label(format!("Duration: {}", duration.as_secs()));
            ui.label(format!("Position: {}", position.as_secs()));
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
