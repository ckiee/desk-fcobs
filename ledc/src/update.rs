use std::{
    io::{stdout, Write},
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Result;
use serialport::SerialPort;

use crate::{open_serial, Controller, SharedAppData, Strip, WaveType};

pub fn update_thread(arc: Arc<Mutex<SharedAppData>>) {
    let mut port = open_serial();
    loop {
        // What we're going to send over the wire.
        let serial_data = {
            thread::sleep(Duration::from_millis(5));
            // prepare the data, copying it (as we serialize?) so we don't hold the lock
            // up as long in case of a deadlock. originally thought this would improve
            // perf in normal cases too, but nope.
            let mut dat = arc.lock().unwrap();

            // Mode-specific logic
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

            let mut out = Vec::with_capacity(24);

            let push_strips = |strips: &[Strip], out: &mut Vec<u8>| {
                strips
                    .iter()
                    // Invert, so our 0 is no light
                    .map(|strip| vec![u16::MAX - strip.0, u16::MAX - strip.1])
                    // Convert to big endian bytes
                    .flat_map(|words| {
                        words
                            .iter()
                            .map(|word| Vec::from(word.to_be_bytes()))
                            .collect::<Vec<_>>()
                    })
                    // Push.
                    .for_each(|mut dat| out.append(&mut dat))
            };

            // out.push(0x3); //IDebugEnable

            if dat.schedule.status_changed {
                dat.schedule.status_changed = false;
                // Are we sending, or cancelling?
                if dat.schedule.send.is_some() {
                    out.push(0x2); // IInterpolateFrame

                    // TODO: Parse these in the UI and have `Duration`s ready to go here.
                    let start = humantime::parse_duration(&dat.schedule.start).unwrap();
                    let length = humantime::parse_duration(&dat.schedule.length).unwrap();

                    out.extend_from_slice(&u32::try_from(start.as_millis()).unwrap().to_be_bytes());
                    out.extend_from_slice(
                        &u32::try_from(length.as_millis()).unwrap().to_be_bytes(),
                    );

                    push_strips(&dat.schedule.endpoint, &mut out); // [Strip]
                } else {
                    out.push(0x4); // INoInterpolate
                }
            }

            // out.push(0x4); // INoInterpolate

            // Selectively push live light data (:
            if match dat.controller {
                Controller::Manual if dat.strips_changed => {
                    dat.strips_changed = false;
                    true
                }
                Controller::Wave { .. } => true,
                _ => false,
            } {
                out.push(0x1); // IImmediate
                push_strips(&dat.strips, &mut out); // [Strip]
            }

            if dat.relay_changed {
                out.push(0x5); // IRelayControl
                out.push(dat.relay_enabled.into()) // bool
            }

            out
        };

        // Send it!
        let send = |port: &mut Box<dyn SerialPort>, serial_data: &Vec<u8>| -> Result<()> {
            port.write_all(serial_data)?;
            let mut buf = vec![0u8; port.bytes_to_read().unwrap() as usize];
            port.read_exact(&mut buf).unwrap();
            stdout().lock().write_all(&buf).unwrap();
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
