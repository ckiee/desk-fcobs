use eframe::egui;
use std::{
    eprintln,
    fs::{self, File},
    io::Write,
    ops::Add,
    path::PathBuf,
    sync::{
        atomic::{self, AtomicBool},
        Arc, Mutex,
    },
    thread::{self, sleep, spawn, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use anyhow::Result;
use app_dirs2::{AppDataType, AppInfo};
use eframe::egui::Slider;
use serde::{Deserialize, Serialize};
use serialport::SerialPort;

use crate::{Controller, SharedAppData, Strip};

impl SharedAppData {
    pub fn new() -> Self {
        SharedAppData {
            strips: vec![Strip(0, 0), Strip(0, 0)],
            controller: Controller::Manual,
            relay_enabled: false,
            relay_changed: false,
        }
    }

    pub fn state_path() -> Result<PathBuf> {
        let info = AppInfo {
            name: "ledc",
            author: "ckie",
        };
        Ok(app_dirs2::app_root(AppDataType::UserConfig, &info)?.with_file_name("state"))
    }

    pub fn load_config() -> Result<Self> {
        let path = Self::state_path()?;
        if !path.try_exists()? {
            let dat = Self::new();
            dat.save_config()?;
            return Ok(dat);
        }

        Ok(bincode::deserialize(&fs::read(path)?)?)
    }

    pub fn save_config(&self) -> Result<()> {
        let path = Self::state_path()?;
        fs::write(path, bincode::serialize(self)?)?;
        Ok(())
    }
}

// This is very overkill. But it's fun (and hopefully efficient)
pub fn config_thread(arc: Arc<Mutex<SharedAppData>>, flag: Arc<AtomicBool>) {
    let mut next_save = Instant::now();
    let debounce_dur = Duration::from_millis(50);
    let mut last_config = None;

    loop {
        // We allow forcing a config save by setting the flag to true and then unparking the thread,
        // Otherwise, we'll just save every 10 min.
        //
        // This might take you a minute:
        // - https://doc.rust-lang.org/std/thread/fn.park_timeout.html
        // - fetch_nand toggles flag.
        while !(flag.fetch_nand(true, atomic::Ordering::SeqCst) && Instant::now() > next_save) {
            thread::park();
        }

        let config = arc.lock().unwrap();
        if Some(config.clone()) != last_config {
            config.save_config().unwrap();
        }

        last_config = Some(config.clone());
        next_save = Instant::now() + debounce_dur;
    }
}
