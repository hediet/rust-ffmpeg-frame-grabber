mod error;
mod ffmpeg;
mod ffprobe;
mod utils;

#[macro_use]
extern crate lazy_static;

pub use error::FFMpegError;
pub use ffmpeg::{FFMpegVideo, FFMpegVideoOptions};
pub use ffprobe::{FFProbeInfo, VideoStreamInfo};
