use std::{
    error::Error,
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
};
use zip::ZipArchive;

pub trait MusicLoader: Send + Sync + 'static {
    type Reader: Read + Seek + Sync + Send;

    fn read(&self) -> Result<LoadedMusic<Self::Reader>, Box<dyn Error>>;
}

#[derive(Clone)]
pub struct LoadedMusic<Reader>
where
    Reader: Read + Seek + Sync + Send,
{
    pub reader: Reader,
}

pub enum FeusicMusicReader {
    ZipFeusic { bytes: Cursor<Vec<u8>> },
    FolderFeusic { bytes: BufReader<File> },
}

#[derive(Debug)]
pub enum FeusicMusicLoader {
    ZipFeusic {
        feusic_path: String,
        music_name: String,
    },
    FolderFeusic {
        music_path: String,
    },
}

impl Read for FeusicMusicReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            FeusicMusicReader::ZipFeusic { bytes } => bytes.read(buf),
            FeusicMusicReader::FolderFeusic { bytes } => bytes.read(buf),
        }
    }
}

impl Seek for FeusicMusicReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            FeusicMusicReader::ZipFeusic { bytes } => bytes.seek(pos),
            FeusicMusicReader::FolderFeusic { bytes } => bytes.seek(pos),
        }
    }
}

impl MusicLoader for FeusicMusicLoader {
    type Reader = FeusicMusicReader;

    fn read(&self) -> Result<LoadedMusic<FeusicMusicReader>, Box<dyn Error>> {
        let reader = match self {
            FeusicMusicLoader::ZipFeusic {
                feusic_path,
                music_name,
            } => {
                let file = File::open(&feusic_path)
                    .map_err(|e| format!("cannot open {}. {}", feusic_path, e))?;
                let mut zip = ZipArchive::new(file)
                    .map_err(|e| format!("cannot open zip file {}. {}", feusic_path, e))?;
                let mut music = zip.by_name(&music_name).map_err(|e| {
                    format!(
                        "cannot find {} in zip file {}. {}",
                        music_name, feusic_path, e
                    )
                })?;

                let mut buf = vec![];
                music.read_to_end(&mut buf)?;

                FeusicMusicReader::ZipFeusic {
                    bytes: Cursor::new(buf),
                }
            }
            FeusicMusicLoader::FolderFeusic { music_path } => {
                let music = File::open(&music_path)
                    .map_err(|e| format!("cannot open {}. {}", music_path, e))?;

                FeusicMusicReader::FolderFeusic {
                    bytes: BufReader::new(music),
                }
            }
        };

        Ok(LoadedMusic { reader })
    }
}
