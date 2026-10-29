use std::sync::Mutex;

use deino_domain::Millis;

use super::*;

/// Answers each stretch with the level its first frame is mapped to.
struct Scripted {
    levels: Vec<(usize, f32)>,
    asked: Mutex<Vec<Range<usize>>>,
    fail: bool,
}

impl Scripted {
    fn new(levels: &[(usize, f32)]) -> Self {
        Self {
            levels: levels.to_vec(),
            asked: Mutex::new(Vec::new()),
            fail: false,
        }
    }
}

impl VoiceCheck for Scripted {
    fn cleaned_peaks(&self, _pcm: &[f32], frames: &[Range<usize>]) -> Result<Vec<f32>, String> {
        self.asked.lock().unwrap().extend(frames.iter().cloned());
        if self.fail {
            return Err("model failed".into());
        }
        Ok(frames
            .iter()
            .map(|f| {
                self.levels
                    .iter()
                    .find(|(at, _)| *at == f.start)
                    .map_or(-20.0, |(_, db)| *db)
            })
            .collect())
    }
}

fn span(start_ms: u32, end_ms: u32, text: &str) -> HeardSpan {
    HeardSpan {
        start: Millis(start_ms),
        end: Millis(end_ms),
        text: text.into(),
        words: Vec::new(),
    }
}

/// Ten seconds: loud speech in 0–2 s, quiet noise everywhere else.
fn track() -> Vec<f32> {
    let mut pcm = vec![0.004_f32; 160_000];
    pcm[..32_000].fill(0.2);
    pcm
}

fn texts(heard: &[HeardSpan]) -> Vec<&str> {
    heard.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn a_quiet_line_with_no_voice_is_dropped_and_speech_is_never_asked_about() {
    let heard = vec![
        span(0, 1_900, "Share my screen."),
        span(3_000, 3_800, "Thank you."),
        span(5_000, 6_500, "Nobody has prepped all homes!"),
        span(8_000, 9_000, "Okay."),
    ];
    let check = Scripted::new(&[(30, -84.0), (50, -76.5), (80, -61.0)]);

    let kept = drop_silent_lines(&track(), heard, &check);

    assert_eq!(
        texts(&kept),
        ["Share my screen.", "Okay."],
        "a quiet line the model hears a voice in stays"
    );
    let asked = check.asked.lock().unwrap().clone();
    assert_eq!(
        asked,
        [30..38, 50..65, 80..90],
        "the loud line never goes to the model"
    );
}

#[test]
fn every_line_is_kept_when_the_model_fails() {
    let heard = vec![span(3_000, 3_800, "Thank you.")];
    let mut check = Scripted::new(&[(30, -90.0)]);
    check.fail = true;

    assert_eq!(
        texts(&drop_silent_lines(&track(), heard, &check)),
        ["Thank you."]
    );
}

#[test]
fn nothing_is_asked_when_every_line_is_loud() {
    let heard = vec![span(0, 1_000, "Hello."), span(1_000, 2_000, "Thanks.")];
    let check = Scripted::new(&[]);

    assert_eq!(drop_silent_lines(&track(), heard, &check).len(), 2);
    assert!(check.asked.lock().unwrap().is_empty());
}
