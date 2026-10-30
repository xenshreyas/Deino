//! The Rust VBx against pyannote's own answers on a synthetic case
//! (`fixtures/vbx-case.json`, made by `scripts/vbx-reference.py`): the PLDA
//! transform, the rough AHC clusters and VBx's responsibilities must match.
//! Needs the bundled PLDA file (Git LFS); without it the test is skipped.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    reason = "an integration-test crate: helpers fail the test by panicking, like #[test] bodies"
)]

use std::path::Path;

use deino_diarize::models::bundled::{FOLDER, VBX};
use deino_diarize::vbx::{Plda, THRESHOLD, VbxParams, ahc, vbx};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    threshold: f64,
    fa: f64,
    fb: f64,
    embeddings: Vec<Vec<f64>>,
    ahc: Vec<usize>,
    fea: Vec<Vec<f64>>,
    gamma: Vec<Vec<f64>>,
    pi: Vec<f64>,
}

fn case() -> Case {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vbx-case.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn plda() -> Option<Plda> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/src-tauri/resources")
        .join(FOLDER)
        .join(VBX.file_name);
    let size = std::fs::metadata(&path).ok()?.len();
    (size == VBX.bytes).then(|| Plda::load(&path).unwrap())
}

#[test]
fn the_rust_vbx_matches_pyannote() {
    let Some(plda) = plda() else {
        println!("skipped: the PLDA file is a Git LFS pointer");
        return;
    };
    let case = case();
    assert!((case.threshold - THRESHOLD).abs() < f64::EPSILON);
    let raw: Vec<Vec<f32>> = case
        .embeddings
        .iter()
        .map(|x| x.iter().map(|v| *v as f32).collect())
        .collect();

    let fea: Vec<Vec<f64>> = raw.iter().map(|x| plda.transform(x)).collect();
    let worst = max_gap(&fea, &case.fea);
    assert!(worst < 1e-4, "PLDA transform differs by {worst}");

    let unit: Vec<Vec<f64>> = case.embeddings.iter().map(|x| unit(x)).collect();
    let rough = ahc::clusters(&unit, THRESHOLD);
    let mapping = same_partition(&rough, &case.ahc);

    let params = VbxParams {
        fa: case.fa,
        fb: case.fb,
        ..VbxParams::default()
    };
    let result = vbx(&case.fea, plda.phi(), &rough, params);
    let gamma: Vec<Vec<f64>> = result
        .gamma
        .iter()
        .map(|row| (0..row.len()).map(|p| row[mapping[p]]).collect())
        .collect();
    let worst = max_gap(&gamma, &case.gamma);
    assert!(worst < 1e-6, "responsibilities differ by {worst}");
    let pi: Vec<f64> = (0..result.pi.len())
        .map(|p| result.pi[mapping[p]])
        .collect();
    assert!(max_gap(&[pi], std::slice::from_ref(&case.pi)) < 1e-6);
}

/// Asserts the two labelings group the points identically; returns, for
/// each of pyannote's labels, the matching Rust label.
fn same_partition(rust: &[usize], python: &[usize]) -> Vec<usize> {
    let clusters = python.iter().max().unwrap() + 1;
    let mut mapping = vec![usize::MAX; clusters];
    for (r, p) in rust.iter().zip(python) {
        assert!(
            mapping[*p] == usize::MAX || mapping[*p] == *r,
            "the AHC clusters differ"
        );
        mapping[*p] = *r;
    }
    assert_eq!(
        rust.iter().max().unwrap() + 1,
        clusters,
        "the AHC cluster counts differ"
    );
    mapping
}

fn max_gap(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .flat_map(|(x, y)| x.iter().zip(y).map(|(p, q)| (p - q).abs()))
        .fold(0.0, f64::max)
}

fn unit(x: &[f64]) -> Vec<f64> {
    let norm = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    x.iter().map(|v| v / norm).collect()
}
