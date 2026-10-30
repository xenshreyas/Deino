//! Community-1's x-vector transform and PLDA (`pyannote.audio.core.plda`):
//! a raw speaker embedding becomes a 128-dimension point in the PLDA space,
//! where VBx models each speaker. The arrays come precomputed from
//! `scripts/export-speaker-models.py` (`community-1-vbx.f64`), so the
//! generalized eigendecomposition pyannote runs at load time is not needed.

/// The embedding size the transform takes (WeSpeaker ResNet34-LM).
pub const INPUT: usize = 256;
/// The PLDA space's size.
pub const DIM: usize = 128;

/// The file's arrays, in order: `mean1`, `lda`, `mean2`, `mu`, `tr`, `psi`.
const SIZES: [usize; 6] = [INPUT, INPUT * DIM, DIM, DIM, DIM * DIM, DIM];

/// Why the file could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PldaError {
    /// The file is not the expected size.
    #[error("the PLDA file holds {actual} bytes where {expected} were expected")]
    Size {
        /// Bytes the arrays need.
        expected: usize,
        /// Bytes found.
        actual: usize,
    },
    /// The file could not be read.
    #[error("the PLDA file could not be read")]
    Io(#[from] std::io::Error),
}

/// The transform, ready to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct Plda {
    mean1: Vec<f64>,
    /// `INPUT × DIM`, row-major.
    lda: Vec<f64>,
    mean2: Vec<f64>,
    mu: Vec<f64>,
    /// `DIM × DIM`, row-major: row `k` is the `k`th basis vector.
    tr: Vec<f64>,
    psi: Vec<f64>,
}

impl Plda {
    /// Reads the bundled file at `path`.
    pub fn load(path: &std::path::Path) -> Result<Self, PldaError> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// Parses the little-endian f64 arrays in `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PldaError> {
        let expected = SIZES.iter().sum::<usize>() * 8;
        if bytes.len() != expected {
            return Err(PldaError::Size {
                expected,
                actual: bytes.len(),
            });
        }
        let mut values = bytes
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap_or_default()));
        let mut take = |count: usize| values.by_ref().take(count).collect::<Vec<f64>>();
        let [mean1, lda, mean2, mu, tr, psi] = SIZES.map(&mut take);
        Ok(Self {
            mean1,
            lda,
            mean2,
            mu,
            tr,
            psi,
        })
    }

    /// The between-speaker variances in the PLDA space (VBx's `Phi`).
    pub fn phi(&self) -> &[f64] {
        &self.psi
    }

    /// `embedding` in the PLDA space: centred, scaled to length √256, through
    /// the LDA, centred, scaled to length √128, then onto the PLDA basis.
    pub fn transform(&self, embedding: &[f32]) -> Vec<f64> {
        let centred: Vec<f64> = embedding
            .iter()
            .zip(&self.mean1)
            .map(|(&x, m)| f64::from(x) - m)
            .collect();
        let scaled = scale_to(&centred, root(INPUT));
        let projected: Vec<f64> = (0..DIM)
            .map(|j| {
                let column = self.lda.iter().skip(j).step_by(DIM);
                column.zip(&scaled).map(|(l, x)| l * x).sum::<f64>()
            })
            .zip(&self.mean2)
            .map(|(x, m)| x - m)
            .collect();
        let xvec = scale_to(&projected, root(DIM));
        let shifted: Vec<f64> = xvec.iter().zip(&self.mu).map(|(x, m)| x - m).collect();
        self.tr
            .chunks_exact(DIM)
            .map(|row| row.iter().zip(&shifted).map(|(t, x)| t * x).sum())
            .collect()
    }
}

/// `vector` scaled to `length` (unchanged when it is zero).
fn scale_to(vector: &[f64], length: f64) -> Vec<f64> {
    let norm = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm == 0.0 {
        return vector.to_vec();
    }
    vector.iter().map(|x| x * length / norm).collect()
}

#[allow(clippy::cast_precision_loss, reason = "256 and 128 are exact in f64")]
fn root(size: usize) -> f64 {
    (size as f64).sqrt()
}
