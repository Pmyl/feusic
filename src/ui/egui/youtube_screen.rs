use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
};

use egui::{Color32, Key};

use crate::{core::youtube::downloader::YoutubeDownloader, ui::Preferences};

pub struct YoutubeScreen {
    url: String,
    downloads: usize,
    download_async_sender: Sender<DownloadResult>,
    download_async_receiver: Receiver<DownloadResult>,
    downloader: Arc<YoutubeDownloader>,
}

enum DownloadResult {
    Error(String),
    Success(String),
}

impl YoutubeScreen {
    pub fn new(_preferences: &Preferences) -> Self {
        let (download_async_sender, download_async_receiver) = channel();
        let current_dir = std::env::current_dir().unwrap().display().to_string();
        YoutubeScreen {
            url: "".to_string(),
            downloads: 0,
            download_async_sender,
            download_async_receiver,
            downloader: Arc::new(
                YoutubeDownloader::new(current_dir).expect("current dir to exists"),
            ),
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Youtube");

            ui.label("Insert Url");
            ui.spacing_mut().text_edit_width = ui.available_width();

            let text_lost_focus = ui.text_edit_singleline(&mut self.url).lost_focus();
            let download_clicked = ui.button("Download").clicked();

            ui.label(format!(
                "All songs will be saved in {}",
                self.downloader.download_dir()
            ));

            if self.downloads > 0 {
                ui.colored_label(
                    Color32::ORANGE,
                    format!("Downloading {} songs...", self.downloads),
                );
                for result in self.download_async_receiver.try_iter() {
                    match result {
                        DownloadResult::Error(error) => {
                            println!("Error downloading audio: {}", error)
                        }
                        DownloadResult::Success(path) => {
                            println!("Audio downloaded with success in path: {}", path)
                        }
                    }
                    self.downloads -= 1;
                }
            } else {
                ui.colored_label(Color32::DARK_GREEN, "Nothing to download");
            }

            if text_lost_focus && ui.ctx().input(|input| input.key_pressed(Key::Enter))
                || download_clicked
            {
                println!("Downloading {}", self.url);
                self.downloads += 1;

                thread::spawn({
                    let url = self.url.clone();
                    let sender = self.download_async_sender.clone();
                    let downloader = self.downloader.clone();
                    move || match downloader.download_audio_blocking(&url) {
                        Ok(path) => sender.send(DownloadResult::Success(path)),
                        Err(error) => sender.send(DownloadResult::Error(format!("{}", error))),
                    }
                });
            }
        });
    }
}
