//! Allocation accounting for the echo canceller in steady state.
//!
//! Deino's wrapper (`SonoraAec`) preallocates every buffer it owns. The
//! `sonora` 0.2.0 port of AEC3 itself still allocates small scratch vectors
//! per 10 ms frame (`sonora_aec3::EchoRemover::process_capture`: 22;
//! `sonora::EchoCanceller3::process_capture_sub_frame`: 8 or so); ADR 0026
//! records the measurement and the follow-up to upstream a scratch-buffer
//! patch. Until then this test pins the count, so a regression (or a fixed
//! upstream release, which should drop it to zero) is noticed. The canceller
//! never runs on a real-time callback, only on the live pipeline's thread.
#![allow(
    unsafe_code,
    reason = "a counting GlobalAlloc must forward to the system allocator"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use deino_recording::echo::{EchoCanceller, FRAME, NoEchoCanceller, SonoraAec};

/// sonora 0.2.0's measured allocations per 10 ms frame, rounded up.
const SONORA_ALLOCATIONS_PER_FRAME: usize = 36;

struct Counting;

// Per thread: the tests run in parallel, and only the allocations of the
// thread being measured count (a shared counter also counted the other
// test's, which made the zero-allocation test fail now and then). Const
// initialised and without `Drop`, so reading them never allocates.
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: forwards every call to the system allocator unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.try_with(Cell::get).unwrap_or(false) {
            // Ignored when the thread is already gone: nothing to count.
            ALLOCATIONS
                .try_with(|count| count.set(count.get() + 1))
                .unwrap_or(());
        }
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn frame(index: usize) -> [f32; FRAME] {
    let mut samples = [0.0; FRAME];
    for (offset, sample) in samples.iter_mut().enumerate() {
        #[allow(clippy::cast_precision_loss, reason = "small test indices")]
        let t = (index * FRAME + offset) as f32 / 16_000.0;
        *sample = 0.2 * (t * 440.0 * std::f32::consts::TAU).sin();
    }
    samples
}

/// Allocations while `canceller` processes frames `range`.
fn count(canceller: &mut dyn EchoCanceller, range: std::ops::Range<usize>) -> usize {
    let frames: Vec<([f32; FRAME], [f32; FRAME])> = range
        .map(|index| (frame(index), frame(index + 3)))
        .collect();
    let mut frames = frames;
    ALLOCATIONS.set(0);
    COUNTING.set(true);
    for (reference, capture) in &mut frames {
        canceller.process(reference, capture);
    }
    COUNTING.set(false);
    ALLOCATIONS.get()
}

#[test]
fn the_pipeline_side_of_echo_handling_allocates_nothing() {
    assert_eq!(count(&mut NoEchoCanceller, 0..1_000), 0);
}

#[test]
fn sonora_allocations_stay_bounded_per_frame_and_do_not_grow() {
    let mut aec = SonoraAec::new();
    count(&mut aec, 0..100);

    let first = count(&mut aec, 100..1_100);
    let second = count(&mut aec, 1_100..2_100);

    assert!(
        first <= 1_000 * SONORA_ALLOCATIONS_PER_FRAME,
        "{first} allocations in 1 000 frames"
    );
    assert!(second <= first, "allocations grew: {first} then {second}");
}
