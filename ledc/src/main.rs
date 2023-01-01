use std::{
    io::Write,
    sync::{Arc, Mutex},
    thread::{self, spawn, JoinHandle, sleep},
    time::{Duration, Instant},
};

use anyhow::Result;
use eframe::egui::{self, Slider};
use serialport::SerialPort;

fn main() {
    let options = eframe::NativeOptions::default();

    eframe::run_native("ledc", options, Box::new(|_cc| Box::new(LedApp::default())));
}

struct Strip(u16, u16);
#[derive(PartialEq)]
enum Controller {
    Manual,
    Wave {
        started_at: Instant,
        interval_ms: f32,
        warm: bool,
        cold: bool,
        ty: WaveType,
    },
}
#[derive(PartialEq, Copy, Clone)]
enum WaveType {
    Sine,
    /// Square wave with duty cycle %
    Square(f32),
}
struct SharedAppData {
    strips: Vec<Strip>,
    controller: Controller,
}
struct LedApp {
    shared: Arc<Mutex<SharedAppData>>,
    #[allow(unused)]
    update_thread: JoinHandle<()>,
    first_render: bool,
    poll_update_fast: bool,
}

fn open_serial() -> Box<dyn SerialPort> {
    let open = || {
        serialport::new(
        "/dev/serial/by-id/usb-Silicon_Labs_CP2102_USB_to_UART_Bridge_Controller_0001-if00-port0",
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
        let shared_dat = Mutex::new(SharedAppData {
            strips: vec![Strip(0, 0), Strip(0, 0)],
            controller: Controller::Manual,
        });

        let uarc = Arc::new(shared_dat);
        let rarc = Arc::clone(&uarc);

        let update_thread = spawn(move || {
            let mut port = open_serial();
            let arc = rarc;
            loop {
                let serial_data = {
                    thread::sleep(Duration::from_millis(5));
                    let mut dat = arc.lock().unwrap();

                    {
                        match dat.controller {
                            Controller::Manual => {}
                            Controller::Wave {
                                started_at,
                                interval_ms,
                                warm,
                                cold,
                                ty,
                            } => {
                                let pos =
                                    started_at.elapsed().as_millis() as f32 / (interval_ms as f32);
                                let pos_mod = pos % 1.0;
                                let u16_max = u16::MAX as f32;
                                let u16_halfmax = u16_max / 2.0;
                                let val = match ty {
                                    WaveType::Sine => pos.sin() * u16_halfmax + u16_halfmax,
                                    WaveType::Square(duty) => {
                                        if pos_mod < duty {
                                            u16_max
                                        } else {
                                            0.0
                                        }
                                    }
                                } as u16;

                                if cold {
                                    dat.strips[0].0 = val;
                                    dat.strips[1].0 = val;
                                }
                                if warm {
                                    dat.strips[0].1 = val;
                                    dat.strips[1].1 = val;
                                }
                            }
                        };
                    }

                    // prepare the data, copying it so we don't hold the lock up as long
                    // in case of a deadlock. originally thought this would improve perf
                    // in normal cases too, but nope.
                    dat.strips
                        .iter()
                        .map(|strip| vec![u16::MAX - strip.0, u16::MAX - strip.1])
                        .collect::<Vec<_>>()
                };

                let send =
                    |port: &mut Box<dyn SerialPort>, serial_data: &Vec<Vec<u16>>| -> Result<()> {
                        port.write(&[0x4, 0x1])?;
                        for d in serial_data {
                            send_immediate(port, &d)?;
                        }
                        Ok(())
                    };
                let mut tries = 0;
                while let Err(err) = send(&mut port, &serial_data) {
                    if tries >= 3 {
                        panic!("serial port write failed too many times: {:?}", err);
                    }
                    // try reopening the port after a bit..
                    sleep(Duration::from_millis(50));
                    port = open_serial();

                    tries += 1;
                }
            }
        });

        Self {
            shared: uarc,
            update_thread,
            first_render: true,
            poll_update_fast: true, // TODO try false for startup cpu% maybe?
        }
    }
}

impl eframe::App for LedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_render {
            ctx.set_pixels_per_point(2.0);
            ctx.request_repaint();
            self.first_render = false;
            return;
        }
        //TODO: doesn't compensate for draw time, rough thing so updates from other thread occur
        let repaint_rate = if self.poll_update_fast {
            144.0 // TODO
        } else {
            1.0
        };
        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / repaint_rate));
        let mut dat = self.shared.lock().unwrap(); // TODO very slow at startup if we happen to be in sync with the data update thread
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ledc");
            ui.group(|ui| {
                ui.label("Control mode");
                ui.radio_value(&mut dat.controller, Controller::Manual, "Manual");
                if ui
                    .radio(matches!(dat.controller, Controller::Wave { .. }), "Wave")
                    .clicked()
                {
                    dat.controller = Controller::Wave {
                        started_at: Instant::now(),
                        interval_ms: 1000.0,
                        warm: true,
                        cold: false,
                        ty: WaveType::Sine,
                    };
                }
            });

            ui.horizontal_wrapped(|ui| {
                for (i, strip) in dat.strips.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label(format!("Strip {i}"));
                            ui.add(Slider::new(&mut strip.0, 0..=65535).text("cold"));
                            ui.add(Slider::new(&mut strip.1, 0..=65535).text("warm"));
                        });
                    });
                }
            });

            self.poll_update_fast = false;
            if let Controller::Wave {
                started_at,
                interval_ms,
                warm,
                cold,
                ty,
            } = &mut dat.controller
            {
                self.poll_update_fast = true;
                ui.group(|ui| {
                    ui.label("Slide controls");
                    let mut immut_pos = (started_at.elapsed().as_millis() as f32) % *interval_ms;
                    ui.add_enabled(
                        false,
                        Slider::new(&mut immut_pos, 0.0..=*interval_ms).text("current"),
                    );
                    ui.group(|ui| {
                        ui.label("Wave type");
                        ui.vertical(|ui| {
                            ui.radio_value(ty, WaveType::Sine, "Sine");

                            if ui
                                .radio(matches!(*ty, WaveType::Square(..)), "Square")
                                .clicked()
                            {
                                *ty = WaveType::Square(0.1);
                            }

                            if let WaveType::Square(duty) = ty {
                                ui.add(Slider::new(duty, 0.0..=1.0).text("duty"));
                            }
                        })
                    });
                    if ui
                        .add(
                            Slider::new(interval_ms, 5.0..=100_000.0)
                                .text("interval")
                                .logarithmic(true),
                        )
                        .changed()
                    {
                        *started_at = Instant::now();
                    }
                    ui.label("Affecting");
                    ui.vertical(|ui| {
                        ui.checkbox(warm, "warm");
                        ui.checkbox(cold, "cold");
                    });
                });
            }
        });
    }
}

// Test:
// 01 ffff ffff ffff ffff
// 01ffffffffffffffff
fn send_immediate(port: &mut Box<dyn SerialPort>, dat: &[u16]) -> Result<()> {
    let mut encoded: Vec<u8> = vec![];
    for v in dat {
        encoded.push((v >> 8) as u8);
        encoded.push((v & 0xff) as u8);
    }
    port.write(&encoded)?;
    Ok(())
}
