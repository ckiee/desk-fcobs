use std::{
    io::Write,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{Duration, SystemTime},
};

use anyhow::Result;
use serialport::SerialPort;

use crate::{open_serial, Controller, SharedAppData, Strip, WaveType};

pub fn update_thread(arc: Arc<Mutex<SharedAppData>>) -> Result<()> {
    let mut port = open_serial();
    let mut status_buf = vec![0u8; 1];
    loop {
        // Parse status_buf
        let animation_running = status_buf[0] != 0;

        // What we're going to send over the wire.
        let (realtime, serial_data) = {
            // Whether the serial_data returned needs to be sent quickly.
            let mut realtime = false;

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
                        let u16_max = f32::from(u16::MAX);
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
                    .for_each(|mut dat| out.append(&mut dat));
            };

            // out.push(0x3); //IDebugEnable

            {
                // TODO: Parse these in the UI and have `Duration`s ready to go here.
                let sched_start = humantime::parse_duration(&dat.schedule.start)?;
                let sched_length = humantime::parse_duration(&dat.schedule.length)?;

                // Reconcile with MCU, swap if animation stopped (probably ended, we hope)
                //
                // - This has to run before we start a new animation.
                // - We're not guaranteed timing for animation_running updates
                //
                {
                    if dat.schedule.send.is_some_and(|t| {
                        SystemTime::now().duration_since(t).is_ok_and(|dur| {
                            dur > ((sched_start + sched_length).saturating_sub(
                                Duration::from_millis(if sched_length > Duration::from_secs(10) {
                                    250
                                } else {
                                    0
                                }),
                            ))
                        })
                    }) && !animation_running
                    {
                        // Swap
                        let prev = dat.strips.clone();
                        dat.strips = dat.schedule.endpoint.clone();
                        dat.schedule.endpoint = prev;
                        // Sync that we're no longer running
                        dat.schedule.send = None;
                    }
                }

                {
                    if dat.schedule.status_changed && dat.schedule.send.is_some() {
                        out.push(0x2); // IInterpolateFrame

                        out.extend_from_slice(
                            &u32::try_from(sched_start.as_millis())?.to_be_bytes(),
                        );
                        out.extend_from_slice(
                            &u32::try_from(sched_length.as_millis())?.to_be_bytes(),
                        );

                        push_strips(&dat.schedule.endpoint, &mut out); // [Strip]
                    }

                    if dat.schedule.send.is_none() {
                        out.push(0x4); // INoInterpolate
                    }

                    dat.schedule.status_changed = false;
                }
            };

            // Selectively push live light data (:
            if match dat.controller {
                Controller::Manual if dat.schedule.send.is_none() => true,
                Controller::Manual if dat.strips_changed => {
                    dat.strips_changed = false;
                    true
                }
                Controller::Wave { .. } => true,
                Controller::Manual => false,
            } {
                out.push(0x1); // IImmediate
                push_strips(&dat.strips, &mut out); // [Strip]
                realtime = true;
            }

            if dat.relay_changed {
                out.push(0x5); // IRelayControl
                out.push(dat.relay_enabled.into()); // bool
            }

            (realtime, out)
        };

        // Send it!
        let send = |port: &mut Box<dyn SerialPort>, serial_data: &Vec<u8>| -> Result<()> {
            port.write_all(serial_data)?;
            // // debugging, forward debug output back out
            // // unwrap_or(0) so we may skip this codepath upon I/O error (e.g. USB plugged transitions)
            // let mut buf = vec![0u8; port.bytes_to_read().unwrap_or(0) as usize];
            // port.read_exact(&mut buf).unwrap();
            // stdout().lock().write_all(&buf).unwrap();

            Ok(())
        };
        let mut tries = 0;
        while let Err(err) = send(&mut port, &serial_data) {
            assert!(tries < 3, "serial port write failed too many times: {err:?}");
            // try reopening the port after a bit..
            sleep(Duration::from_millis(50));
            port = open_serial();

            tries += 1;
        }

        // `realtime` doesn't do much, so I assume checking /just/ the last frame
        // is not sufficient. But either way it's okay.
        if !realtime {
            // Lastly, let's sneak a bit of data back out. No more error handling here, probably fine.
            // IReadStatus
            if port.write_all(&[0x6]).is_ok() {
                // expect one byte back, but it's okay if we don't get it (see: USB disconnect)
                port.read_exact(&mut status_buf).ok();
            }
        }
    }
}
