use std::sync::Mutex;

use super::*;
use crate::echo::SonoraAec;

const RATE: usize = 16_000;

/// Deterministic speech-like noise (the far end): white noise from a
/// linear congruential generator under a syllable-rate envelope.
fn far_end(len: usize) -> Vec<f32> {
    let mut state: u32 = 7;
    (0..len)
        .map(|i| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            #[allow(clippy::cast_precision_loss, reason = "test signal values are small")]
            let (noise, t) = ((state >> 8) as f32 / 8_388_608.0 - 1.0, i as f32 / 16_000.0);
            0.3 * (0.5 + 0.5 * (t * 4.0 * std::f32::consts::TAU).sin()) * noise
        })
        .collect()
}

/// The user's voice: a 220 Hz tone during `seconds`, silence otherwise.
fn owner(len: usize, seconds: std::ops::Range<usize>) -> Vec<f32> {
    (0..len)
        .map(|i| {
            #[allow(clippy::cast_precision_loss, reason = "test signal values are small")]
            let t = i as f32 / 16_000.0;
            if seconds.contains(&(i / RATE)) {
                0.2 * (t * 220.0 * std::f32::consts::TAU).sin()
            } else {
                0.0
            }
        })
        .collect()
}

fn energy(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s * s).sum()
}

#[test]
fn the_calls_echo_is_removed_from_the_microphone_and_the_user_kept() {
    let len = 10 * RATE;
    let far = far_end(len);
    let voice = owner(len, 7..9);
    let delay = 800; // 50 ms from the speakers to the microphone
    let mic: Vec<f32> = (0..len)
        .map(|i| voice[i] + i.checked_sub(delay).map_or(0.0, |j| far[j] * 0.5))
        .collect();
    let mut cleaned = mic.clone();
    let seen = Mutex::new(Vec::new());

    let done = cancel_echo(&mut cleaned, &far, &mut SonoraAec::new(), &|f| {
        seen.lock().unwrap().push(f);
        true
    });

    assert!(done);
    // Seconds 4–7: only echo, after the canceller has converged.
    let echo = 4 * RATE..7 * RATE;
    let drop_db = 10.0 * (energy(&mic[echo.clone()]) / energy(&cleaned[echo]).max(1e-12)).log10();
    assert!(drop_db > 15.0, "echo down by {drop_db:.1} dB");
    // Seconds 7–9: the user's voice over the echo is still there.
    let spoken = 7 * RATE..9 * RATE;
    let kept = energy(&cleaned[spoken.clone()]) / energy(&voice[spoken]);
    assert!(kept > 0.3, "user's voice kept: {kept:.2}");
    let seen = seen.into_inner().unwrap();
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(seen.last(), Some(&1.0));
}

#[test]
fn a_short_reference_and_a_partial_last_frame_are_padded() {
    let mut mic = vec![0.1; FRAME * 3 + 37];
    let reference = vec![0.0; FRAME];

    assert!(cancel_echo(
        &mut mic,
        &reference,
        &mut crate::echo::NoEchoCanceller,
        &|_| true
    ));

    assert!(mic.iter().all(|s| (s - 0.1).abs() < f32::EPSILON));
}

#[test]
fn a_stop_from_progress_ends_the_pass_early() {
    let mut mic = vec![0.1; RATE * 5];
    let reference = vec![0.0; RATE * 5];
    let calls = Mutex::new(0);

    let done = cancel_echo(
        &mut mic,
        &reference,
        &mut crate::echo::NoEchoCanceller,
        &|_| {
            *calls.lock().unwrap() += 1;
            false
        },
    );

    assert!(!done);
    assert_eq!(calls.into_inner().unwrap(), 1);
}
