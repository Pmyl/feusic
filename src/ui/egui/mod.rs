use egui::Slider;

use crate::core::{
    feusic::loader::MusicLoader, player::controller::FeusicPlayerController,
    playlist::loader::FolderPlaylistLoader,
};
use std::{error::Error, time::Duration};

mod view;

const TITLE: &str = "Feusic Player";

struct FeusicEguiApp<M: MusicLoader, P: FolderPlaylistLoader<M>> {
    player: FeusicPlayerController<M>,
    playlist_loader: P,
    pixel_per_point: f32,
}

impl<M: MusicLoader, P: FolderPlaylistLoader<M>> eframe::App for FeusicEguiApp<M, P> {
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

                    if ui.button("Select playlist folder…").clicked() {
                        self.player.pause();

                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            match self.playlist_loader.load(path.to_str().unwrap()) {
                                Ok(playlist) => {
                                    self.player.set_playlist(playlist);
                                    self.player.play();
                                }
                                Err(e) => {
                                    self.player.resume();
                                    eprintln!("Error loading playlist: {}", e);
                                }
                            }
                        } else {
                            self.player.resume();
                        }
                    }
                });
            });
        });
    }
}

pub fn run_ui<M: MusicLoader, P: FolderPlaylistLoader<M>>(
    player: FeusicPlayerController<M>,
    playlist_loader: P,
) -> Result<(), Box<dyn Error>> {
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
                playlist_loader,
                pixel_per_point: 2.0,
            }))
        }),
    )?;

    Ok(())
}
