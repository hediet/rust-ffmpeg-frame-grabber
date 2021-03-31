use crate::error::{CommandSpawnError, FFMpegError, IOError};
use crate::utils::{fractional_from_str, from_str};
use io::Read;
use serde::Deserialize;
use snafu::ResultExt;
use std::result::Result;
use std::{collections::HashMap, io::BufReader};
use std::{fmt::Debug, time::Duration};
use std::{io, path::PathBuf};
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub struct FFProbeInfo {
    pub duration: Duration,
    streams: Vec<StreamInfo>,
}

enum StreamInfo {
    Video(VideoStreamInfo),
    // TODO to be extended with AudioStreamInfo
}

#[derive(Clone, Debug)]
pub struct VideoStreamInfo {
    /// The width of each frame.
    pub width: u32,

    // The height of each frames.
    pub height: u32,

    // The frame rate of this stream.
    pub frame_rate: f64,

    /// The total count of frames in this stream as set in the metadata.
    /// The actual count of frames that can be read might differ.
    pub frames_count: u64,
}

impl FFProbeInfo {
    pub fn of(
        input_video_path: &Path,
        ffprobe_path: Option<PathBuf>,
    ) -> Result<FFProbeInfo, FFMpegError> {
        let output = FFProbeOutput::of(input_video_path, ffprobe_path)?;
        Ok(FFProbeInfo {
            duration: Duration::from_secs_f64(output.format.duration),
            streams: output
                .streams
                .iter()
                .filter(|s| s.width.is_some() && s.height.is_some())
                .map(|s| {
                    StreamInfo::Video(VideoStreamInfo {
                        width: s.width.unwrap(),
                        height: s.height.unwrap(),
                        frame_rate: s.avg_frame_rate,
                        frames_count: s.nb_frames,
                    })
                })
                .collect(),
        })
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    #[allow(unreachable_patterns)]
    pub fn primary_video_stream(&self) -> Option<&VideoStreamInfo> {
        let video_streams = self
            .streams
            .iter()
            .filter_map(|s| match s {
                StreamInfo::Video(v) => Some(v),
                _ => None,
            })
            .collect::<Vec<_>>();

        if video_streams.len() == 1 {
            video_streams.first().cloned()
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug)]
struct FFProbeOutput {
    streams: Vec<FFProbeStreamInfo>,
    format: FFProbeFormat,
}

impl FFProbeOutput {
    pub fn of(
        input_video_path: &Path,
        ffprobe_path: Option<PathBuf>,
    ) -> Result<FFProbeOutput, FFMpegError> {
        if !input_video_path.exists() {
            return Err(FFMpegError::FileDoesNotExistsError {
                file: input_video_path.to_path_buf(),
            });
        }

        let mut cmd =
            Command::new(ffprobe_path.map_or("ffprobe".to_owned(), |p| p.to_string_lossy().into()))
                .args(&[
                    "-v",
                    "error",
                    "-show_entries",
                    "stream",
                    "-show_entries",
                    "format",
                    "-of",
                    "json",
                    &input_video_path.to_string_lossy(),
                ])
                .stdout(Stdio::piped())
                .spawn()
                .context(CommandSpawnError)?;

        let stdout = cmd.stdout.as_mut().unwrap();
        let mut stdout_reader = BufReader::new(stdout);

        let mut json: String = String::new();
        stdout_reader.read_to_string(&mut json).context(IOError)?;

        let output: FFProbeOutput = match serde_json::from_str(&json) {
            Ok(e) => e,
            Err(_) => return Err(FFMpegError::ParseError),
        };
        Ok(output)
    }
}

#[derive(Deserialize, Debug)]
struct FFProbeStreamInfo {
    codec_name: String,
    codec_type: String,

    width: Option<u32>,
    height: Option<u32>,

    #[serde(deserialize_with = "fractional_from_str")]
    r_frame_rate: f64,
    #[serde(deserialize_with = "fractional_from_str")]
    avg_frame_rate: f64,
    #[serde(deserialize_with = "from_str")]
    nb_frames: u64,
}

#[derive(Deserialize, Debug)]
struct FFProbeFormat {
    #[serde(deserialize_with = "from_str")]
    duration: f64,
    tags: HashMap<String, String>,
}
