use std::{
    error::Error,
    fs::remove_file,
    path::Path,
    process::Command,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

use egui::{Color32, Key};

use crate::ui::Preferences;

pub struct YoutubeScreen {
    url: String,
    downloads: usize,
    download_async_sender: Sender<DownloadResult>,
    download_async_receiver: Receiver<DownloadResult>,
    download_dir: String,
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
            download_dir: current_dir,
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Youtube");

            ui.label("Insert Url");
            ui.spacing_mut().text_edit_width = ui.available_width();

            let text_lost_focus = ui.text_edit_singleline(&mut self.url).lost_focus();
            let download_clicked = ui.button("Download").clicked();

            ui.label(format!("All songs will be saved in {}", self.download_dir));

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
                    let download_dir = self.download_dir.clone();
                    move || {
                        let download = || -> Result<String, Box<dyn Error>> {
                            let video = rusty_ytdl::blocking::Video::new(&url).map_err(|e| {
                                format!("Cannot get video from url {}. Error {}", url, e)
                            })?;
                            let video_path = Path::new(&download_dir).join(filenamify::filenamify(
                                video.get_info()?.video_details.title,
                            ));
                            let audio_path = format!("{}.mp3", video_path.display());
                            video.download(&video_path)?;
                            Command::new("ffmpeg")
                                .arg("-i")
                                .arg(&video_path)
                                .arg(&audio_path)
                                .spawn()
                                .map_err(|e| format!("Error spawning command. Error: {}", e))?
                                .wait()
                                .map_err(|e| {
                                    format!("Error executing ffmpeg mp3 conversion. Error: {}", e)
                                })?;
                            remove_file(video_path)?;

                            return Ok(audio_path);
                        };

                        match download() {
                            Ok(path) => sender.send(DownloadResult::Success(path)),
                            Err(error) => sender.send(DownloadResult::Error(format!("{}", error))),
                        }
                    }
                });
            }
        });
    }
}
