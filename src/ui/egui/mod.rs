use egui::Slider;

use crate::core::{
    feusic::loader::MusicLoader, player::controller::FeusicPlayerController,
    playlist::loader::FolderPlaylistLoader,
};
use std::error::Error;

use super::{Preferences, PreferencesHandler};

mod controls;
mod extras;
mod playlist;

const TITLE: &str = "Feusic Player";

struct FeusicEguiApp<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler> {
    player: FeusicPlayerController<M>,
    playlist_loader: P,
    pixel_per_point: f32,
    preferences_handler: PH,
    preferences: Preferences,
}

impl<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler> eframe::App
    for FeusicEguiApp<M, P, PH>
{
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.pixel_per_point);
        egui::TopBottomPanel::top("Size controls").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.pixel_per_point, 1.0..=4.0));
        });
        egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
            extras::render(
                ui,
                &self.player,
                &self.playlist_loader,
                &mut self.preferences,
                &self.preferences_handler,
            );
        });
        egui::TopBottomPanel::bottom("Player controls").show(ctx, |ui| {
            controls::render(ui, &self.player);
            ui.add_space(5.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Playlist");
            playlist::render(ui, &self.player);
        });
    }
}

pub fn run_ui<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler>(
    player: FeusicPlayerController<M>,
    playlist_loader: P,
    preferences: Preferences,
    preferences_handler: PH,
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
                preferences,
                preferences_handler,
            }))
        }),
    )?;

    Ok(())
}
