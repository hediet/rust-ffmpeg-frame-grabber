use crate::ffprobe::FFProbeInfo;
use crate::{
    error::{CommandSpawnError, FFMpegError, IOError},
    ffprobe::VideoStreamInfo,
};
use image::{ImageBuffer, Rgb};
use regex::Regex;
use snafu::ResultExt;
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::result::Result;
use std::str::FromStr;
use std::time::Duration;
use std::{collections::HashMap, path::Path};
use std::{
    io::{BufRead, BufReader, ErrorKind, Read},
    path::PathBuf,
};

pub struct FFMpegVideo {
    stdout: ChildStdout,
    stderr: BufReader<ChildStderr>,
    info: FFProbeInfo,
    primary_video_stream_info: VideoStreamInfo,
}

#[derive(Default)]
pub struct FFMpegVideoOptions {
    sampling_interval: Option<Duration>,
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
}

impl FFMpegVideoOptions {
    pub fn with_sampling_interval(self, sampling_interval: Duration) -> Self {
        FFMpegVideoOptions {
            sampling_interval: Some(sampling_interval),
            ..self
        }
    }

    pub fn with_ffmpeg_path(self, path: PathBuf) -> Self {
        FFMpegVideoOptions {
            ffmpeg_path: Some(path),
            ..self
        }
    }

    pub fn with_ffprobe_path(self, path: PathBuf) -> Self {
        FFMpegVideoOptions {
            ffprobe_path: Some(path),
            ..self
        }
    }
}

impl FFMpegVideo {
    pub fn open(video_path: &Path, options: FFMpegVideoOptions) -> Result<Self, FFMpegError> {
        let info = FFProbeInfo::of(video_path, options.ffprobe_path)?;

        let mut cmd = Command::new(
            options
                .ffmpeg_path
                .map_or("ffmpeg".to_owned(), |p| p.to_string_lossy().into()),
        );
        cmd.args(&["-i", &video_path.to_string_lossy()]);

        let mut filters = Vec::<String>::new();

        if let Some(interval) = options.sampling_interval {
            filters.push(format!("fps=1/{:?}", interval.as_secs()));
        }

        filters.push("showinfo".to_owned());

        cmd.args(&["-vf", &filters.join(",")]);

        cmd.args(&[
            "-f",
            "image2pipe",
            "-an", // disable audio processing
            "-sn", // disable sub-title processing
            "-pix_fmt",
            "rgb24",
            "-nostats",
            "-vcodec",
            "rawvideo",
            "-",
        ]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().context(CommandSpawnError)?;

        let stdout = child.stdout.unwrap();
        let stderr = child.stderr.unwrap();

        let primary_video_stream_info = info
            .primary_video_stream()
            .ok_or(FFMpegError::ParseError)?
            .clone();

        Ok(FFMpegVideo {
            stdout,
            stderr: BufReader::new(stderr),
            info,
            primary_video_stream_info,
        })
    }

    pub fn duration(&self) -> Duration {
        self.info.duration
    }
}

pub struct Frame {
    /// The decoded image.
    pub image: FrameBuffer,

    /// The offset of this frame in the video. Might not be the true time offset.
    pub time_offset: Duration,
}

pub type FrameBuffer = ImageBuffer<Rgb<u8>, Vec<u8>>;

impl FFMpegVideo {
    fn get_next(&mut self) -> Result<Option<Frame>, FFMpegError> {
        let mut infos = std::collections::HashMap::new();
        let mut line = String::new();

        // There are two show info lines.
        // If we don't read all of them, the stream will block.
        let mut lines_to_read = 2;
        while lines_to_read > 0 {
            line.clear();
            self.stderr.read_line(&mut line).context(IOError)?;
            if line.len() == 0 {
                return Ok(None);
            }
            if parse_showinfo(&line, &mut infos).is_some() {
                lines_to_read = lines_to_read - 1;
            }
        }
        let time_seconds = f64::from_str(infos.get("pts_time").unwrap()).unwrap();

        let i = &self.primary_video_stream_info;
        let mut buffer = vec![0u8; (i.width * i.height * 3) as usize];

        if let Err(err) = self.stdout.read_exact(&mut buffer) {
            if err.kind() == ErrorKind::UnexpectedEof {
                // Indicates the last frame has been read.
                return Ok(None);
            }
            return Err(FFMpegError::IOError { source: err });
        }

        let image = FrameBuffer::from_raw(
            self.primary_video_stream_info.width,
            self.primary_video_stream_info.height,
            buffer,
        )
        .expect("Buffer to have correct size");

        Ok(Some(Frame {
            image,
            time_offset: Duration::from_secs_f64(time_seconds),
        }))
    }
}

impl Iterator for FFMpegVideo {
    type Item = Result<Frame, FFMpegError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next() {
            Err(err) => Some(Err(err)),
            Ok(Some(val)) => Some(Ok(val)),
            Ok(None) => None,
        }
    }
}

fn parse_showinfo(line: &str, props: &mut HashMap<String, String>) -> std::option::Option<()> {
    if !line.starts_with(&"[Parsed_showinfo_") {
        return None;
    }

    if line.contains("] config") {
        return None;
    }

    lazy_static! {
        static ref RE: Regex = Regex::new(r"((?P<key>\w+):\s*(?P<value>(\[.*?]|\S+)))").unwrap();
    }

    for cap in RE.captures_iter(line) {
        let key = cap.name("key").unwrap().as_str();
        let name = cap.name("value").unwrap().as_str();
        props.insert(key.to_string(), name.to_string());
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use insta::{assert_json_snapshot, Settings};
    use std::collections::HashMap;

    fn test_parse_show_info(line: &str) -> Option<HashMap<String, String>> {
        let mut map = HashMap::new();
        super::parse_showinfo(line, &mut map).map(|_| map)
    }

    pub fn setup() -> () {
        let mut s = Settings::new();
        s.set_sort_maps(true);
        s.bind_to_thread();
    }

    #[test]
    fn test_parse_showinfo1() {
        setup();
        assert_json_snapshot!(
            test_parse_show_info(
                "[Parsed_showinfo_1 @ 000002669dfefec0] n:   1 pts:      1 pts_time:120     pos: 14185698 fmt:yuv420p sar:1/1 s:1920x1080 i:P iskey:0 type:B checksum:A91F982B plane_checksum:[7BFA6F14 ED4B1E62 92900AB5] mean:[227 127 130] stdev:[35.1 10.3 10.0]",
            ),
            @r###"
        {
          "checksum": "A91F982B",
          "fmt": "yuv420p",
          "i": "P",
          "iskey": "0",
          "mean": "[227 127 130]",
          "n": "1",
          "plane_checksum": "[7BFA6F14 ED4B1E62 92900AB5]",
          "pos": "14185698",
          "pts": "1",
          "pts_time": "120",
          "s": "1920x1080",
          "sar": "1/1",
          "stdev": "[35.1 10.3 10.0]",
          "type": "B"
        }
        "###
        );
    }

    #[test]
    fn test_parse_showinfo2() {
        setup();
        assert_json_snapshot!(
            test_parse_show_info(
                "[Parsed_showinfo_1 @ 000002669dfefec0] color_range:unknown color_space:unknown color_primaries:unknown color_trc:unknown"
            ),
            @r###"
        {
          "color_primaries": "unknown",
          "color_range": "unknown",
          "color_space": "unknown",
          "color_trc": "unknown"
        }
        "###
        );
    }

    #[test]
    fn test_parse_showinfo3() {
        setup();
        assert_json_snapshot!(
            test_parse_show_info("Output #0, image2pipe, to 'pipe:':"),
            @"null"
        );
    }

    #[test]
    fn test_parse_showinfo4() {
        setup();
        assert_json_snapshot!(
            test_parse_show_info(
                "[Parsed_showinfo_1 @ 000002669dfefec0] config in time_base: 120/1, frame_rate: 1/120",
            ),
            @"null"
        );
    }

    #[test]
    fn test_parse_showinfo5() {
        setup();
        assert_json_snapshot!(
            test_parse_show_info(
                "[Parsed_showinfo_1 @ 000002669dfefec0] config out time_base: 0/0, frame_rate: 0/0",
            ),
            @"null"
        );
    }
}
