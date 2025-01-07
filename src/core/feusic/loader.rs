use std::{
    error::Error,
    fs::File,
    io::{BufReader, Cursor, Read},
    time::Duration,
};

use rodio::{Decoder, Sink, Source};
use zip::ZipArchive;

pub trait MusicLoader: Send + Sync + 'static {
    fn load_to_sink(&self, sink: &Sink) -> Result<Duration, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct FesicMusicLoader {
    pub fesic_path: String,
    pub music_name: String,
}

impl MusicLoader for FesicMusicLoader {
    fn load_to_sink(&self, sink: &Sink) -> Result<Duration, Box<dyn Error>> {
        let file = File::open(&self.fesic_path)
            .map_err(|e| format!("cannot open {}. {}", self.fesic_path, e))?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| format!("cannot open zip file {}. {}", self.fesic_path, e))?;
        let mut music = zip.by_name(&self.music_name).map_err(|e| {
            format!(
                "cannot find {} in zip file {}. {}",
                self.music_name, self.fesic_path, e
            )
        })?;

        let mut buf = vec![];
        music.read_to_end(&mut buf)?;

        let source = Decoder::new(BufReader::new(Cursor::new(buf)))?;
        let total_duration = source.total_duration();
        sink.append(source);

        Ok(total_duration.unwrap_or(Duration::from_secs(0)))
    }
}
