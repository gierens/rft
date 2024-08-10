use crate::loss_simulation::LossSimulation;

pub struct Server {
    port: u16,
    #[allow(dead_code)]
    loss_sim: Option<LossSimulation>,
}

impl Server {
    pub fn new(port: u16, loss_sim: Option<LossSimulation>) -> Self {
        Server { port, loss_sim }
    }

    pub fn run(&self) {
        println!("Server running on port {}", self.port);
    }
}
