use super::*;

/// Three seconds of a far end and a microphone that hears it plus a voice.
fn tracks() -> (Vec<f32>, Vec<f32>) {
    let samples = 48_000_u16;
    let far: Vec<f32> = (0..samples)
        .map(|n| (f32::from(n) * 0.031).sin() * 0.3)
        .collect();
    let mic = far
        .iter()
        .zip(0..samples)
        .map(|(echo, n)| echo * 0.6 + (f32::from(n) * 0.007).sin() * 0.2)
        .collect();
    (mic, far)
}

fn whole_pass(mut mic: Vec<f32>, far: &[f32]) -> Vec<f32> {
    assert!(cancel_echo(&mut mic, far, &mut SonoraAec::new(), &|_| true));
    mic
}

#[test]
fn a_pass_during_the_meeting_and_one_after_cancel_as_one_pass_does() {
    let (mic, far) = tracks();
    let memory = PassMemory::new();
    let (mut early_mic, early_far) = (mic[..32_000].to_vec(), far[..32_000].to_vec());
    let mut late = mic.clone();

    assert!(cancel_resumed(
        &memory.following(),
        (&mut early_mic, &early_far),
        &|_| true
    ));
    assert!(cancel_resumed(&memory, (&mut late, &far), &|_| true));

    assert_eq!(late, whole_pass(mic, &far));
    assert!(
        memory.echo().is_none(),
        "the pass after the meeting takes it"
    );
}

#[test]
fn a_start_that_changed_is_cancelled_again_from_the_beginning() {
    let (mic, far) = tracks();
    let memory = PassMemory::new();
    let mut early: Vec<f32> = mic[..32_000].iter().map(|sample| sample * 0.5).collect();
    let mut late = mic.clone();

    assert!(cancel_resumed(
        &memory.following(),
        (&mut early, &far[..32_000]),
        &|_| true
    ));
    assert!(cancel_resumed(&memory, (&mut late, &far), &|_| true));

    assert_eq!(late, whole_pass(mic, &far));
}
