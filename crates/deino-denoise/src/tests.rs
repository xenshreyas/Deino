use super::*;

/// Keyboard-like clicks: a short decaying burst every 150 ms over room hiss.
fn clicks(seconds: usize) -> Vec<f32> {
    let mut seed = 7_u32;
    let mut noise = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from(u16::try_from(seed >> 16).unwrap_or(0)) / 32_768.0 - 1.0
    };
    (0..seconds * SAMPLE_RATE_HZ)
        .map(|at| {
            let since_click = at % 2_400;
            let envelope = if since_click < 160 {
                0.3 * (1.0 - f32::from(u16::try_from(since_click).unwrap()) / 160.0)
            } else {
                0.0
            };
            noise() * (envelope + 0.002)
        })
        .collect()
}

#[test]
fn levels_are_read_on_the_100_ms_grid() {
    let mut pcm = vec![0.0_f32; FRAME * 4];
    pcm[FRAME * 2..FRAME * 3].fill(0.5);
    assert!(peak_db(&pcm, 0..2) < -170.0, "silence");
    assert!((peak_db(&pcm, 0..4) - 20.0 * 0.5_f32.log10()).abs() < 0.01);
    assert_eq!(
        frames_of(1_250, 1_380),
        12..13,
        "a span inside one frame is judged on it"
    );
    assert_eq!(frames_of(1_250, 2_380), 12..23);
}

#[test]
fn clicks_clean_to_near_silence() {
    let pcm = clicks(6);
    let frames = 40..50;
    let raw = peak_db(&pcm, frames.clone());
    let cleaned = Cleaner::new().unwrap().cleaned_peak(&pcm, frames).unwrap();
    assert!(
        raw > -35.0,
        "the clicks are as loud as typing near a laptop mic: {raw}"
    );
    assert!(
        cleaned < raw - 20.0,
        "and the model removes them: {raw} -> {cleaned}"
    );
}

#[test]
fn frames_past_the_end_are_silent() {
    let pcm = clicks(1);
    let cleaned = Cleaner::new().unwrap().cleaned_peak(&pcm, 50..60).unwrap();
    assert!(cleaned < -170.0);
}
