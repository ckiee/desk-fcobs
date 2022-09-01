use std::{
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};

use eframe::egui;
use serialport::SerialPort;

fn main() {
    // let mut port = serialport::new("/dev/ttyUSB1", 115_200)
    //     .timeout(Duration::from_millis(10))
    //     .open()
    //     .expect("Failed to open port");
    let clock = Instant::now();
    // loop {
    //     let sine = |freq: f32, phase: f32| {
    //         let t = ((clock.elapsed().as_secs_f32() + phase) * freq).sin();
    //         ((t + 0.5) * 65535.0) as u16
    //     };

    //     send(&mut port, &[
    //         sine(1.5, 0.0),
    //         sine(1.5, 0.5),
    //         sine(1.2, 1.0),
    //         sine(1.2, 1.5),
    //     ]);

    //     thread::sleep(Duration::from_millis(5));
    //     // > 115200bps/8/4/2
    //     // 1800
    //     // > 1/1800
    //     // 0.0005555555555555556
    // }
    let options = eframe::NativeOptions::default();
    eframe::run_native("ledc", options, Box::new(|_cc| Box::new(MyApp::default())));
}

struct Strip(u16, u16);
struct MyApp {
    strips: Vec<Strip>,
    port: Box<dyn SerialPort>,
}

impl MyApp {
    fn send_state(&mut self) {
        for strip in &self.strips {
            let v = vec![strip.1, strip.0]; // TODO &[]
            send(&mut self.port, &v);
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            strips: vec![Strip(0, 0), Strip(0, 0)],
            port: serialport::new("/dev/ttyUSB1", 115_200)
                .timeout(Duration::from_millis(10))
                .open()
                .expect("Failed to open port"),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ledc");
            // ui.horizontal(|ui| {
            //     ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name);
            // });
            for strip in &mut self.strips {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Slider::new(&mut strip.0, 0..=65535).text("cold"));
                    ui.add(egui::Slider::new(&mut strip.1, 0..=65535).text("warm"));
                });
            }
            self.send_state();
            // if ui.button("Click each year").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}
fn send(port: &mut Box<dyn SerialPort>, dat: &[u16]) {
    dbg!(&dat);
    let mut encoded: Vec<u8> = vec![];
    for v in dat {
        encoded.push((v >> 8) as u8);
        encoded.push((v & 0xff) as u8);
    }
    port.write(&encoded).unwrap();
}
