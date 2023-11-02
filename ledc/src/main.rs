use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::{sleep, spawn, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use serde::{Deserialize, Serialize};
use serialport::SerialPort;

mod config;
mod ui;
mod update;

fn main() {
    let options = eframe::NativeOptions::default();

    eframe::run_native("ledc", options, Box::new(|_cc| Box::<LedApp>::default()));
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Strip(u16, u16);
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
enum Controller {
    Manual,
    Wave {
        #[serde(skip)]
        #[serde(default = "Instant::now")] // gets thrown away anyway
        started_at: Instant,
        interval_ms: f32,
        warm: bool,
        cold: bool,
        ty: WaveType,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
enum WaveType {
    Sine,
    /// Square wave with duty cycle %
    Square(f32),
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct ScheduleUi {
    start: String,
    length: String,
    endpoint: Vec<Strip>,
    send: Option<SystemTime>, // TODO: Set to none once `length` has passed.
    status_changed: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SharedAppData {
    strips: Vec<Strip>,
    strips_changed: bool,
    controller: Controller,
    relay_enabled: bool,
    relay_changed: bool,
    schedule: ScheduleUi,
}

struct LedApp {
    shared: Arc<Mutex<SharedAppData>>,
    #[allow(unused)]
    update_thread: JoinHandle<()>,
    config_thread: JoinHandle<()>,
    first_render: bool,
    poll_update_fast: bool,
    config_thread_flag: Arc<AtomicBool>,
}

pub fn open_serial() -> Box<dyn SerialPort> {
    let open = || {
        serialport::new(
            "/dev/serial/by-id/usb-Silicon_Labs_CP2102_ckie_desk-fcobs_LED_control_0001-if00-port0",
            115_200,
        )
        .timeout(Duration::from_millis(500))
        .open()
    };

    let mut tries = 0;
    loop {
        let result = open();
        match result {
            Ok(port) => return port,
            Err(err) => {
                eprintln!("[try {tries}] serial port open failed: {:?}", err);
                tries += 1;
                sleep(Duration::from_millis(100));
            }
        }
    }
}

impl Default for LedApp {
    fn default() -> Self {
        let shared_dat = Mutex::new(SharedAppData::load_config().unwrap());

        let display_arc = Arc::new(shared_dat);
        let update_arc = Arc::clone(&display_arc);
        let config_arc = Arc::clone(&display_arc);

        let update_thread = spawn(move || update::update_thread(update_arc).unwrap());
        let config_thread_flag = Arc::new(AtomicBool::new(true));
        let config_thread_flag2 = Arc::clone(&config_thread_flag);
        let config_thread = spawn(move || config::config_thread(config_arc, config_thread_flag2));

        Self {
            shared: display_arc,
            update_thread,
            config_thread,
            config_thread_flag,
            first_render: true,
            poll_update_fast: true, // TODO try false for startup cpu% maybe?
        }
    }
}
