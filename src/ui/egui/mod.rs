use egui::IconData;
use youtube_screen::YoutubeScreen;

use crate::core::{
    feusic::loader::MusicLoader, player::controller::FeusicPlayerController,
    playlist::loader::FolderPlaylistLoader,
};
use std::error::Error;

use super::{Preferences, PreferencesHandler};

mod controls;
mod extras;
mod playlist;
mod tabs;
mod youtube_screen;

const TITLE: &str = "Feusic Player";
const ICON: &[u8; 2478] = include_bytes!("../../../assets/yunaka.png");

struct FeusicEguiApp<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler> {
    player: FeusicPlayerController<M>,
    playlist_loader: P,
    preferences_handler: PH,
    preferences: Preferences,

    youtube_screen: Option<YoutubeScreen>,
    screen: FeusicEguiScreen,
}

enum FeusicEguiScreen {
    Main,
    Youtube,
}

impl<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler> eframe::App
    for FeusicEguiApp<M, P, PH>
{
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(pixels_per_point) = self.preferences.pixels_per_point {
            ctx.set_pixels_per_point(pixels_per_point);
        }

        egui::TopBottomPanel::top("Tabs").show(ctx, |ui| {
            if let Some(new_screen) = tabs::render(ui, &self.screen) {
                self.screen = new_screen;

                match (&self.screen, &self.youtube_screen) {
                    (FeusicEguiScreen::Youtube, None) => {
                        self.youtube_screen = Some(YoutubeScreen::new(&self.preferences))
                    }
                    _ => {}
                }
            }
        });

        match self.screen {
            FeusicEguiScreen::Main => self.render_main(ctx),
            FeusicEguiScreen::Youtube => {
                if let Some(screen) = self.youtube_screen.as_mut() {
                    screen.render(ctx)
                }
            }
        }
    }
}

impl<M: MusicLoader, P: FolderPlaylistLoader<M>, PH: PreferencesHandler> FeusicEguiApp<M, P, PH> {
    fn render_main(&mut self, ctx: &egui::Context) {
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
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(ICON)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_transparent(true)
            .with_icon(IconData {
                rgba: icon_rgba,
                width: icon_width,
                height: icon_height,
            }),
        ..Default::default()
    };

    eframe::run_native(
        TITLE,
        options,
        Box::new(|_| {
            Ok(Box::new(FeusicEguiApp {
                player,
                playlist_loader,
                preferences,
                preferences_handler,
                youtube_screen: None,
                screen: FeusicEguiScreen::Main,
            }))
        }),
    )?;

    Ok(())
}
