use eframe::egui;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, sleep, spawn, JoinHandle},
    time::{Duration, Instant},
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
