use snafu::Snafu;
use std::io;
use std::{
    fmt::{Debug, Display, Formatter},
    path::PathBuf,
};

impl Debug for FFMpegError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Snafu)]
#[snafu(visibility = "pub")]
pub enum FFMpegError {
    #[snafu(display("File '{}' does not exists.", file.to_string_lossy()))]
    FileDoesNotExistsError { file: PathBuf },
    #[snafu(display("Could not parse ffmpeg/ffprobe output."))]
    ParseError,
    #[snafu(display("Failed to spawn ffmpeg/ffprobe. {}", source))]
    CommandSpawnError { source: io::Error },
    #[snafu(display("Failed to read data from ffmpeg/ffprobe. {}", source))]
    IOError { source: io::Error },
}
