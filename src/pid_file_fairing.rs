use rocket::fairing::{self, Fairing};
use std::fs::File;
use std::process;
use std::io::Write;
use crate::SiteConfig;

pub struct PidFileFairing;

#[rocket::async_trait]
impl Fairing for PidFileFairing {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Write PID file on launch",
            kind: fairing::Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &rocket::Rocket<rocket::Orbit>) {
        let config = rocket.state::<SiteConfig>();
        if let Some(config) = config {
            if config.pid_file != "" {
                File::create(&config.pid_file).unwrap().write_all(process::id().to_string().as_bytes()).unwrap();
            }
        }
    }
}
