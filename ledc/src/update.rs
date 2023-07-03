use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Result;
use serialport::SerialPort;

use crate::{open_serial, Controller, SharedAppData, WaveType};

pub fn update_thread(arc: Arc<Mutex<SharedAppData>>) {
    let mut port = open_serial();
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
                        let pos = started_at.elapsed().as_millis() as f32 / interval_ms;
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

            let mut out = Vec::with_capacity(24);

            // set the led strip's states
            out.push(0x4);
            out.push(0x1);
            dat.strips
                .iter()
                .map(|strip| vec![u16::MAX - strip.0, u16::MAX - strip.1])
                .flat_map(|words| {
                    words
                        .iter()
                        .map(|word| vec![(word >> 8) as u8, (word & 0xff) as u8])
                        .collect::<Vec<_>>()
                })
                .for_each(|mut dat| out.append(&mut dat));

            if dat.relay_changed {
                out.push(0x5);
                out.push(if dat.relay_enabled { 255 } else { 0 });
            }

            out
        };

        let send = |port: &mut Box<dyn SerialPort>, serial_data: &Vec<u8>| -> Result<()> {
            port.write_all(serial_data)?;
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
}
