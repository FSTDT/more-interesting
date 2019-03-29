use rocket::fairing::{self, Fairing};
use std::fs::File;
use std::process;
use std::io::Write;

pub struct PidFileFairing;

impl Fairing for PidFileFairing {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Write PID file on launch",
            kind: fairing::Kind::Launch,
        }
    }

    fn on_launch(&self, rocket: &rocket::Rocket) {
        let config = rocket.config();
        if let Ok(pid_file) = config.get_str("pid_file") {
            File::create(pid_file).unwrap().write_all(process::id().to_string().as_bytes()).unwrap();
        }
    }
}
