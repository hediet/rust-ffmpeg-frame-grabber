# FFmpeg Rust Adapter

## Installation

```
cargo add ffmpeg_frame_grabber
```

## Requirements

This library requires the `ffmpeg` and `ffprobe` commands to be installed and in path!

## Usage

```rust
use ffmpeg_frame_grabber::{FFMpegVideo, FFMpegVideoOptions};
use image_visualizer::{visualizer::view, VisualizableImage};
use std::{path::Path, time::Duration};

fn main()s {
    let video = FFMpegVideo::open(
        Path::new(&"./data/video.mp4"),
        FFMpegVideoOptions::default().with_sampling_interval(Duration::from_secs(120)),
    )
    .unwrap();

    for frame in video {
        let f = frame.unwrap();
        println!("offset: {:?}", f.time_offset);
        view!(&f.image.visualize());
    }
}
```
