use std::{
    fs::{self},
    path::PathBuf,
    sync::{
        atomic::{self, AtomicBool},
        Arc, Mutex,
    },
    thread::{self},
    time::{Duration, Instant},
};

use anyhow::Result;
use app_dirs2::{AppDataType, AppInfo};

use crate::{Controller, ScheduleUi, SharedAppData, Strip};

impl SharedAppData {
    pub fn new() -> Self {
        SharedAppData {
            strips: vec![Strip(0, 0), Strip(0, 0)],
            // Yes, the program just started, so the strips *have* changed from
            // their previous, unknown state.
            strips_changed: true,
            controller: Controller::Manual,
            relay_enabled: false,
            relay_changed: false,
            schedule: ScheduleUi {
                start: ("6h30m".to_string(), None),
                length: ("30m".to_string(), None),
                endpoint: vec![Strip(u16::MAX, 0); 2],
                send: None,
                status_changed: false,
                swap_on_stop: false,
            },
        }
    }

    pub fn state_path() -> Result<PathBuf> {
        let info = AppInfo {
            name: "ledc",
            author: "ckie",
        };
        Ok(app_dirs2::app_root(AppDataType::UserConfig, &info)?.join("state"))
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

impl Default for SharedAppData {
    fn default() -> Self {
        Self::new()
    }
}

// This is very overkill. But it's fun (and hopefully efficient)
pub fn config_thread(arc: Arc<Mutex<SharedAppData>>, flag: Arc<AtomicBool>) {
    let mut next_save = Instant::now();
    let debounce_dur = Duration::from_millis(50);
    let mut last_config = None;

    loop {
        // We allow forcing a config save by setting the flag to true and then unparking the thread,
        // Otherwise, we'll just wake up /about/ every second, when the eframe repaint triggers us.
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
