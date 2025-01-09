use crate::core::{feusic::loader::MusicLoader, player::FeusicPlayerController};
use std::error::Error;

mod view;

const TITLE: &str = "Hello, egui!";

struct FeusicEguiApp<M: MusicLoader> {
    player: FeusicPlayerController<M>,
}

impl<M: MusicLoader> eframe::App for FeusicEguiApp<M> {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(3.0);
        egui::CentralPanel::default().show(ctx, |ui| -> Result<(), Box<dyn Error>> {
            view::render(&ui, &self.player).into()
        });
    }
}

pub fn run_ui<M: MusicLoader>(player: FeusicPlayerController<M>) -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([1024.0, 768.0])
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        TITLE,
        options,
        Box::new(|_| Ok(Box::new(FeusicEguiApp { player }))),
    )?;

    Ok(())
}
