//! A manual check of [`deino_audio::mux`] (CAP-09), not run in CI: gives a
//! silent video the sound of an audio file, then reports whether each file
//! has an audio track.
//!
//! ```sh
//! cargo run -p deino-audio --example mux_probe -- video.mp4 audio.m4a out.mp4
//! ```

#![allow(clippy::print_stdout, reason = "a manual probe reports on stdout")]

use std::path::PathBuf;

use deino_audio::mux::{MuxRequest, add_audio, has_audio};

fn main() {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let [video, audio, destination] = args.as_slice() else {
        return println!("usage: mux_probe <video.mp4> <audio.m4a> <out.mp4>");
    };
    println!("video has audio: {:?}", has_audio(video));
    let result = add_audio(&MuxRequest {
        video,
        audio,
        destination,
    });
    println!("mux: {result:?}");
    println!("output has audio: {:?}", has_audio(destination));
}
