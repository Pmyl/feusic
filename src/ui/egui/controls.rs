use std::time::Duration;

use egui::{style::HandleShape, Ui};

use crate::core::{feusic::loader::MusicLoader, player::controller::FeusicPlayerController};

pub(super) fn render<M: MusicLoader>(ui: &mut Ui, player: &FeusicPlayerController<M>) {
    ui.ctx().request_repaint_after(Duration::from_millis(200));

    let duration = player.music_duration();
    let position = player.music_position();
    let music_index = player.music_index();
    let mut relative_position = position.as_millis() as f32 / duration.as_millis() as f32;

    ui.vertical_centered(|ui| {
        let music_names = player.music_names();
        for (i, name) in music_names.get().iter().enumerate() {
            if i == music_index {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(20.0, eframe::epaint::FontFamily::Proportional),
                );
                ui.label(name);
                ui.reset_style();
            } else {
                ui.label(name);
            }
        }
    });

    ui.separator();

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

        ui.label(format!(
            "{} / {}",
            format_duration(position),
            format_duration(duration)
        ));
    });
}

fn format_duration(duration: Duration) -> String {
    let s = duration.as_secs();
    let (h, s) = (s / 3600, s % 3600);
    let (m, s) = (s / 60, s % 60);
    format!("{:02}:{:02}:{:02}", h, m, s)
}
