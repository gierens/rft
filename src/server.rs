pub struct Server {
    port: u16,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Server { port }
    }

    pub fn run(&self) {
        println!("Server running on port {}", self.port);
    }
}
