//! VBx (Landini et al., "Bayesian HMM clustering of x-vector sequences"),
//! as `pyannote.audio.utils.vbx.VBx` and `cluster_vbx` run it for
//! Community-1: each speaker is a Gaussian in the PLDA space, the
//! responsibilities start from the first pass's clusters (one-hot, softened
//! by a softmax at 7), and speakers no point needs fade to a zero prior.
//! The equation numbers are the paper's.

/// The tuning Community-1 ships.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VbxParams {
    /// Scales the sufficient statistics.
    pub fa: f64,
    /// Speaker regularisation: higher keeps fewer speakers.
    pub fb: f64,
    /// The most iterations.
    pub max_iters: usize,
}

impl Default for VbxParams {
    fn default() -> Self {
        Self {
            fa: 0.07,
            fb: 0.8,
            max_iters: 20,
        }
    }
}

/// How sharply the first pass's clusters start the responsibilities.
const INIT_SMOOTHING: f64 = 7.0;
/// Stop once the objective improves by less.
const EPSILON: f64 = 1e-4;

/// The result: each point's responsibilities (`points × speakers`) and each
/// speaker's prior.
#[derive(Clone, Debug, PartialEq)]
pub struct Vbx {
    /// `gamma[t][s]`: how much point `t` belongs to speaker `s`.
    pub gamma: Vec<Vec<f64>>,
    /// `pi[s]`: the speaker's share; near zero for a dropped speaker.
    pub pi: Vec<f64>,
}

/// Runs VBx on `x` (points in the PLDA space) with between-speaker
/// variances `phi`, starting from the first-pass labels `init`.
pub fn vbx(x: &[Vec<f64>], phi: &[f64], init: &[usize], params: VbxParams) -> Vbx {
    let speakers = init.iter().max().map_or(0, |max| max + 1);
    let mut gamma: Vec<Vec<f64>> = init.iter().map(|&s| initial(s, speakers)).collect();
    let mut pi = vec![1.0 / count(speakers); speakers];
    let dims = phi.len();
    let constant: Vec<f64> = x
        .iter()
        .map(|point| -0.5 * (dot(point, point) + count(dims) * (2.0 * std::f64::consts::PI).ln()))
        .collect();
    let rho: Vec<Vec<f64>> = x
        .iter()
        .map(|point| point.iter().zip(phi).map(|(v, p)| v * p.sqrt()).collect())
        .collect();
    let mut previous: Option<f64> = None;
    for _ in 0..params.max_iters {
        let model = Model::fit(&gamma, &rho, phi, params);
        let (next, log_px) = model.responsibilities(&rho, &constant, &pi);
        gamma = next;
        pi = priors(&gamma, speakers);
        let elbo = log_px + params.fb * 0.5 * model.penalty();
        if previous.is_some_and(|before| elbo - before < EPSILON) {
            break;
        }
        previous = Some(elbo);
    }
    Vbx { gamma, pi }
}

/// The speaker models given the responsibilities: (16) and (17).
struct Model {
    alpha: Vec<Vec<f64>>,
    inv_l: Vec<Vec<f64>>,
    /// `0.5 (invL + alpha²) · Phi` per speaker, the data-free part of (23).
    fixed: Vec<f64>,
    fa: f64,
}

impl Model {
    fn fit(gamma: &[Vec<f64>], rho: &[Vec<f64>], phi: &[f64], params: VbxParams) -> Self {
        let speakers = gamma.first().map_or(0, Vec::len);
        let ratio = params.fa / params.fb;
        let mut alpha = Vec::with_capacity(speakers);
        let mut inv_l = Vec::with_capacity(speakers);
        for s in 0..speakers {
            let weight: f64 = gamma.iter().map(|row| at(row, s)).sum();
            let inv: Vec<f64> = phi
                .iter()
                .map(|p| 1.0 / (1.0 + ratio * weight * p))
                .collect();
            let stats = (0..phi.len()).map(|d| {
                gamma
                    .iter()
                    .zip(rho)
                    .map(|(g, r)| at(g, s) * at(r, d))
                    .sum::<f64>()
            });
            alpha.push(stats.zip(&inv).map(|(sum, i)| ratio * i * sum).collect());
            inv_l.push(inv);
        }
        let fixed = alpha
            .iter()
            .zip(&inv_l)
            .map(|(a, i): (&Vec<f64>, &Vec<f64>)| {
                0.5 * a
                    .iter()
                    .zip(i)
                    .zip(phi)
                    .map(|((a, i), p)| (i + a * a) * p)
                    .sum::<f64>()
            })
            .collect();
        Self {
            alpha,
            inv_l,
            fixed,
            fa: params.fa,
        }
    }

    /// New responsibilities, (23), and the total log-likelihood.
    fn responsibilities(
        &self,
        rho: &[Vec<f64>],
        constant: &[f64],
        pi: &[f64],
    ) -> (Vec<Vec<f64>>, f64) {
        let log_pi: Vec<f64> = pi.iter().map(|p| (p + 1e-8).ln()).collect();
        let (fixed, fa) = (&self.fixed, self.fa);
        let mut total = 0.0;
        let gamma = rho
            .iter()
            .zip(constant)
            .map(|(r, g)| {
                let scores: Vec<f64> = self
                    .alpha
                    .iter()
                    .zip(fixed)
                    .zip(&log_pi)
                    .map(|((a, f), lp)| fa * (dot(r, a) - f + g) + lp)
                    .collect();
                let norm = log_sum_exp(&scores);
                total += norm;
                scores.iter().map(|s| (s - norm).exp()).collect()
            })
            .collect();
        (gamma, total)
    }

    /// The speaker-model part of the objective, (25).
    fn penalty(&self) -> f64 {
        self.alpha
            .iter()
            .zip(&self.inv_l)
            .flat_map(|(a, i)| a.iter().zip(i))
            .map(|(a, i)| i.ln() - i - a * a + 1.0)
            .sum()
    }
}

/// The one-hot start for `speaker`, softened: softmax(7 · one-hot).
fn initial(speaker: usize, speakers: usize) -> Vec<f64> {
    let high = INIT_SMOOTHING.exp();
    let total = high + count(speakers.saturating_sub(1));
    (0..speakers)
        .map(|s| {
            if s == speaker {
                high / total
            } else {
                1.0 / total
            }
        })
        .collect()
}

fn priors(gamma: &[Vec<f64>], speakers: usize) -> Vec<f64> {
    let sums: Vec<f64> = (0..speakers)
        .map(|s| gamma.iter().map(|row| at(row, s)).sum())
        .collect();
    let total: f64 = sums.iter().sum();
    sums.iter()
        .map(|s| s / total.max(f64::MIN_POSITIVE))
        .collect()
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if max.is_infinite() {
        return max;
    }
    max + values.iter().map(|v| (v - max).exp()).sum::<f64>().ln()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn at(row: &[f64], index: usize) -> f64 {
    row.get(index).copied().unwrap_or(0.0)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "speaker and dimension counts are tiny"
)]
fn count(value: usize) -> f64 {
    value as f64
}
