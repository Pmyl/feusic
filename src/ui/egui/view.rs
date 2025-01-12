use std::{error::Error, time::Duration};

use egui::{style::HandleShape, Ui};

use crate::core::{feusic::loader::MusicLoader, player::controller::FeusicPlayerController};

pub(super) fn render<M: MusicLoader>(
    ui: &Ui,
    player: &FeusicPlayerController<M>,
) -> Result<(), Box<dyn Error>> {
    egui::Window::new("Controls")
        .open(&mut true)
        .show(&ui.ctx(), |ui| {
            ui.ctx().request_repaint_after(Duration::from_millis(200));

            let duration = player.music_duration();
            let position = player.music_position();

            let mut relative_position = position.as_millis() as f32 / duration.as_millis() as f32;

            ui.label(format!("Duration: {}", duration.as_millis()));
            ui.label(format!("Position: {}", position.as_millis()));
            ui.separator();

            if ui.button("Crossfade").clicked() {
                player.crossfade(Duration::from_millis(1000));
            }

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(false, |ui| ui.button("<<"));

                    if player.paused() {
                        if ui.button("|>").clicked() {
                            player.resume();
                        }
                    } else {
                        if ui.button("||").clicked() {
                            player.pause();
                        }
                    }

                    if ui.button(">>").clicked() {
                        player.next();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                let old_pos = relative_position;
                let slider = ui.add(
                    egui::widgets::Slider::new(&mut relative_position, 0.0..=1.0)
                        .handle_shape(HandleShape::Rect { aspect_ratio: 0.3 })
                        .show_value(false),
                );

                if slider.drag_stopped() && old_pos != relative_position {
                    player.seek(player.music_duration().mul_f32(relative_position));
                }
            });
        });

    Ok(())
}
