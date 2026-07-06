use tracing::info;

pub struct RelaySimulator {
    is_on: bool,
}

impl RelaySimulator {
    pub fn new() -> Self {
        info!("relay initialized — defaulting to ON");
        Self { is_on: true }
    }

    pub fn keep_on(&mut self) {
        if !self.is_on {
            info!("relay toggled ON");
            self.is_on = true;
        }
    }

    pub fn cut_off(&mut self) {
        if self.is_on {
            info!("relay toggled OFF (cut-off)");
            self.is_on = false;
        }
    }

    #[allow(dead_code)]
    pub fn is_on(&self) -> bool {
        self.is_on
    }
}
