use std::{error::Error, time::Duration};

use egui::{style::HandleShape, Ui};

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

            ui.label(format!("Duration: {}", duration.as_millis()));
            ui.label(format!("Position: {}", position.as_millis()));
            ui.separator();
            ui.label("Controls");
            ui.separator();

            if ui.button("Crossfade").clicked() {
                player.crossfade(Duration::from_millis(1000));
            }

            if ui.button("Seek to 1 sec left").clicked() {
                player.seek_to_one_second_left();
            }

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(false, |ui| ui.button("<<"));
                    ui.add_enabled_ui(false, |ui| ui.button("<"));

                    if player.paused() {
                        if ui.button("|>").clicked() {
                            player.resume();
                        }
                    } else {
                        if ui.button("||").clicked() {
                            player.pause();
                        }
                    }

                    if ui.button(">").clicked() {
                        player.next_repeat();
                    }

                    if ui.button(">>").clicked() {
                        player.next();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.add(
                    egui::widgets::Slider::new(&mut relative_position, 0.0..=1.0)
                        .handle_shape(HandleShape::Rect { aspect_ratio: 0.3 })
                        .show_value(false),
                );
            });
        });

    Ok(())
}
