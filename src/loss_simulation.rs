use rand::{rngs::ThreadRng, Rng};

#[allow(dead_code)]
pub struct LossSimulation {
    rng: ThreadRng,
    p: f64,
    q: f64,
    /// true with current packet is to be lost
    state: bool,
}

#[allow(dead_code)]
impl LossSimulation {
    pub fn new(p: f64, q: f64) -> Self {
        LossSimulation {
            rng: rand::thread_rng(),
            p,
            q,
            state: false,
        }
    }

    pub fn from_options(p: Option<f64>, q: Option<f64>) -> Option<Self> {
        match (p, q) {
            (Some(p), Some(q)) => Some(Self::new(p, q)),
            (Some(p), None) => Some(Self::new(p, p)),
            (None, Some(q)) => Some(Self::new(q, q)),
            _ => None,
        }
    }

    pub fn next(&mut self) -> bool {
        let prob = if self.state { self.q } else { self.p };
        self.state = self.rng.gen_bool(prob);
        self.state
    }
}
