//! Compiles kaldi-native-fbank 1.20.1 and the `knfc` C shim into a static
//! library. Upstream knf-rs drove `CMake` and `bindgen`; the shim's two functions
//! are declared by hand in `src/knf_rs.rs`, so `cc` is all the build needs.

/// The kaldi-native-fbank sources the filterbank pulls in. `online-feature.cc`
/// instantiates the MFCC and Whisper extractors too, so they are compiled.
const SOURCES: [&str; 10] = [
    "feature-fbank.cc",
    "feature-functions.cc",
    "feature-mfcc.cc",
    "feature-window.cc",
    "fftsg.cc",
    "kaldi-math.cc",
    "mel-computations.cc",
    "online-feature.cc",
    "rfft.cc",
    "whisper-feature.cc",
];

fn main() {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .include(".")
        .include("knfc")
        .file("knfc/knfc.cc")
        // The extractor runs for every diarization window, so it is optimised
        // even in debug builds.
        .opt_level(2)
        // Third-party code: its warnings are not ours to fix.
        .warnings(false);
    for source in SOURCES {
        build.file(format!("kaldi-native-fbank/csrc/{source}"));
    }
    build.compile("knfc");
    println!("cargo:rerun-if-changed=kaldi-native-fbank");
    println!("cargo:rerun-if-changed=knfc");
}
