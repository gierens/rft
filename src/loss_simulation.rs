use rand::{rngs::ThreadRng, Rng};

#[derive(Debug, PartialEq)]
enum LossState {
    NotLost,
    Lost,
}

pub struct LossSimulation {
    rng: ThreadRng,
    p: f64,
    q: f64,
    state: LossState,
}

impl LossSimulation {
    pub fn new(p: f64, q: f64) -> Self {
        LossSimulation {
            rng: rand::thread_rng(),
            p,
            q,
            state: LossState::NotLost,
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
        let prob = match self.state {
            LossState::NotLost => self.p,
            LossState::Lost => self.q,
        };
        self.state = if self.rng.gen_bool(prob) {
            LossState::Lost
        } else {
            LossState::NotLost
        };
        self.state == LossState::Lost
    }
}
