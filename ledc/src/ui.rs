use eframe::egui::{self, Grid, InnerResponse, TextEdit, Ui};
use std::{
    sync::atomic,
    time::{Duration, Instant, SystemTime},
};

use eframe::egui::Slider;

use crate::{Controller, LedApp, Strip, WaveType};

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
            1.0 // this minimal repaint is reused for the config mechanism, see end of this function.
        };
        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / repaint_rate));
        let mut dat = self.shared.lock().unwrap(); // TODO very slow at startup if we happen to be in sync with the data update thread
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ledc");

            ui.horizontal_wrapped(|ui| {
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
                if ui.checkbox(&mut dat.relay_enabled, "Relay").changed() {
                    dat.relay_changed = true;
                }
            });

            let make_strip_controls =
                |ui: &mut Ui, strips: &mut Vec<Strip>| -> InnerResponse<bool> {
                    ui.horizontal_wrapped(|ui| {
                        let mut changed = false;
                        for (i, strip) in strips.iter_mut().enumerate() {
                            changed |= ui
                                .vertical(|ui| {
                                    ui.group(|ui| {
                                        ui.label(format!("Strip {i}"));
                                        let mut changed = false;
                                        changed |= ui
                                            .add(Slider::new(&mut strip.0, 0..=65535).text("cold"))
                                            .changed();
                                        changed |= ui
                                            .add(Slider::new(&mut strip.1, 0..=65535).text("warm"))
                                            .changed();
                                        changed
                                    })
                                    .inner
                                })
                                .inner;
                        }
                        changed
                    })
                };

            dat.strips_changed |= make_strip_controls(ui, &mut dat.strips).inner;

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

            ui.group(|ui| {
                let btn = ui.selectable_label(dat.schedule.send.is_some(), "Schedule");
                if btn.clicked() {
                    if dat.schedule.send.is_none() {
                        dat.schedule.send = Some(SystemTime::now());
                    } else {
                        // Force push of controller state.
                        dat.schedule.send = None;
                        dat.strips_changed = true;
                    }
                    dat.schedule.status_changed = true;
                } else if btn.secondary_clicked() {
                    // Swap
                    let prev = dat.strips.clone();
                    dat.strips = dat.schedule.endpoint.clone();
                    dat.schedule.endpoint = prev;
                }

                if {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            Grid::new("schedule_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let mut changed = false;
                                    ui.label("begin after");
                                    changed |= ui
                                        .add(
                                            TextEdit::singleline(&mut dat.schedule.start)
                                                .desired_width(80.),
                                        )
                                        .changed();
                                    ui.end_row();

                                    ui.label("transition length");
                                    changed |= ui
                                        .add(
                                            TextEdit::singleline(&mut dat.schedule.length)
                                                .desired_width(80.),
                                        )
                                        .changed();
                                    ui.end_row();

                                    changed
                                })
                                .inner
                        })
                        .inner
                    })
                    .inner
                } || make_strip_controls(ui, &mut dat.schedule.endpoint).inner
                {
                    dat.schedule.send = None;
                }
            });
        });

        // The user is probably touching us, let's save the config.
        //
        // update gets called at least once a second.
        if !self.poll_update_fast {
            self.config_thread_flag
                .store(true, atomic::Ordering::Release);
            self.config_thread.thread().unpark();
        }
    }
}
