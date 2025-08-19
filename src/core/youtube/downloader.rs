use std::{error::Error, fs::remove_file, path::Path, process::Command};

pub struct YoutubeDownloader {
    download_dir: String,
}

impl YoutubeDownloader {
    pub fn new(download_dir: String) -> Result<Self, Box<dyn Error>> {
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| format!("{e}. -> When creating download dir"))?;
        Ok(Self { download_dir })
    }

    pub fn download_dir(&self) -> &str {
        &self.download_dir
    }

    pub fn download_audio_blocking(&self, url: &str) -> Result<String, Box<dyn Error>> {
        self.download_audio_blocking_with_filename(url, "")
    }

    pub fn download_audio_blocking_with_filename(
        &self,
        url: &str,
        filename_prefix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let video = rusty_ytdl::blocking::Video::new(url)
            .map_err(|e| format!("Cannot get video from url {}. Error {}", url, e))?;
        let video_path = Path::new(&self.download_dir).join(filenamify::filenamify(
            video.get_info()?.video_details.title,
        ));
        let mut audio_path = video_path.clone();
        audio_path.set_file_name(format!(
            "{}{}.mp3",
            filename_prefix,
            audio_path.file_name().unwrap().display()
        ));
        let audio_path = audio_path.display().to_string();
        video
            .download(&video_path)
            .map_err(|e| format!("{e}. -> When downloading video."))?;
        Command::new("ffmpeg")
            .arg("-i")
            .arg(&video_path)
            .arg(&audio_path)
            .spawn()
            .map_err(|e| format!("Error spawning command. Error: {}", e))?
            .wait()
            .map_err(|e| format!("Error executing ffmpeg mp3 conversion. Error: {}", e))?;
        remove_file(video_path).map_err(|e| format!("{e}. -> When removing video."))?;

        Ok(audio_path)
    }
}
