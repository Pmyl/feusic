use egui::Slider;

use crate::core::player::controller::FeusicPlayerController;
use std::{error::Error, time::Duration};

mod view;

const TITLE: &str = "Feusic Player";

struct FeusicEguiApp {
    player: FeusicPlayerController,
    pixel_per_point: f32,
}

impl eframe::App for FeusicEguiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.pixel_per_point);
        egui::TopBottomPanel::bottom("Feusic Player").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.pixel_per_point, 1.0..=4.0));
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Window::new("Controls")
                .title_bar(false)
                .default_width(ui.available_width() * 0.8)
                .default_pos((ui.available_width() * 0.1, ui.available_height() * 0.1))
                .show(&ui.ctx(), |ui| view::render(ui, &self.player));

            egui::Window::new("Extras").show(&ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Crossfade").clicked() {
                        self.player.crossfade(Duration::from_millis(1000));
                    }

                    if ui.button("Remove loop").clicked() {
                        self.player.remove_loop();
                    }
                });
            });
        });
    }
}

pub fn run_ui(player: FeusicPlayerController) -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        TITLE,
        options,
        Box::new(|_| {
            Ok(Box::new(FeusicEguiApp {
                player,
                pixel_per_point: 2.0,
            }))
        }),
    )?;

    Ok(())
}
