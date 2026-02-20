use std::time::Instant;

use beamng_rs::sensors::{Camera, CameraConfig};
use beamng_rs::vehicle::{Vehicle, VehicleOptions};
use beamng_rs::{BeamNg, Scenario};
use eframe::egui;
use tokio::sync::mpsc;

const W: usize = 1280;
const H: usize = 720;

/// Simulation steps per camera frame. Higher = faster sim, fewer network round-trips.
const STEPS_PER_FRAME: u32 = 3;

struct Frame {
    rgba: Vec<u8>,
    frame_ms: f64,
}

enum ControlCmd {
    Drive {
        steering: f64,
        throttle: f64,
        brake: f64,
        parkingbrake: f64,
    },
}

struct App {
    frame_rx: mpsc::Receiver<Frame>,
    control_tx: mpsc::Sender<ControlCmd>,
    texture: Option<egui::TextureHandle>,
    fps: f64,
    frame_ms: f64,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain channel, keep only the latest frame
        let mut latest = None;
        while let Ok(f) = self.frame_rx.try_recv() {
            latest = Some(f);
        }

        // Upload to GPU texture and update stats
        if let Some(frame) = latest {
            self.frame_ms = frame.frame_ms;
            if frame.frame_ms > 0.0 {
                self.fps = 1000.0 / frame.frame_ms;
            }
            let img = egui::ColorImage::from_rgba_unmultiplied([W, H], &frame.rgba);
            match &mut self.texture {
                Some(h) => h.set(img, egui::TextureOptions::LINEAR),
                None => {
                    self.texture = Some(ctx.load_texture("cam", img, egui::TextureOptions::LINEAR));
                }
            }
        }

        // Read keyboard state
        let (fwd, back, left, right, space) = ctx.input(|i| {
            (
                i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp),
                i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown),
                i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft),
                i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight),
                i.key_down(egui::Key::Space),
            )
        });
        let steering = if left {
            -1.0
        } else if right {
            1.0
        } else {
            0.0
        };
        let throttle = if fwd { 1.0 } else { 0.0 };
        let brake = if back { 1.0 } else { 0.0 };
        let parkingbrake = if space { 1.0 } else { 0.0 };
        let _ = self.control_tx.try_send(ControlCmd::Drive {
            steering,
            throttle,
            brake,
            parkingbrake,
        });

        // Display camera feed with FPS overlay
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                let response = ui.image(tex);

                // FPS overlay in top-left corner of the image
                let rect = response.rect;
                let painter = ui.painter();
                let text = format!("{:.0} fps  |  {:.1} ms", self.fps, self.frame_ms);
                painter.text(
                    rect.left_top() + egui::vec2(8.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::monospace(16.0),
                    egui::Color32::from_rgba_unmultiplied(0, 255, 0, 200),
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Connecting to BeamNG...");
                });
            }
        });

        ctx.request_repaint();
    }
}

/// Convert raw colour bytes to RGBA suitable for egui.
fn colour_to_rgba(colour: Vec<u8>) -> Option<Vec<u8>> {
    let expected_rgb = W * H * 3;
    let expected_rgba = W * H * 4;
    if colour.len() == expected_rgb {
        let mut buf = Vec::with_capacity(expected_rgba);
        for px in colour.chunks_exact(3) {
            buf.extend_from_slice(px);
            buf.push(255);
        }
        Some(buf)
    } else if colour.len() == expected_rgba {
        let mut buf = colour;
        for px in buf.chunks_exact_mut(4) {
            px[3] = 255;
        }
        Some(buf)
    } else {
        None
    }
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    let (frame_tx, frame_rx) = mpsc::channel::<Frame>(2);
    let (control_tx, mut control_rx) = mpsc::channel(16);

    // Background thread: tokio runtime with BeamNG connection
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut bng = BeamNg::new("192.168.1.85", 5555).connect().await.unwrap();
                println!("Connected to BeamNG.tech!");

                // let _ = bng.control().return_to_main_menu().await;
                let _ = Scenario::delete(
                    &mut bng,
                    "/levels/italy/scenarios/manual_control_gui/manual_control_gui.json",
                )
                .await;

                let mut scenario = Scenario::new("italy", "manual_control_gui");
                scenario.add_vehicle(
                    "ego",
                    "etk800",
                    (237.90, -894.42, 246.10),
                    (0.0173, -0.0019, -0.6354, 0.7720),
                    VehicleOptions {
                        color: Some((1.0, 1.0, 1.0, 1.0)),
                        ..Default::default()
                    },
                );
                scenario.make(&mut bng).await.unwrap();
                println!("Scenario created.");

                let mut ego = Vehicle::new("ego", "etk800");
                bng.settings()
                    .set_deterministic(Some(60), None)
                    .await
                    .unwrap();
                bng.scenario()
                    .load_scenario(&scenario, true, &mut [&mut ego])
                    .await
                    .unwrap();
                bng.scenario().start(false).await.unwrap();
                bng.control().pause().await.unwrap();
                println!("Scenario started (deterministic, paused).");

                let camera = Camera::open(
                    "camera1",
                    &mut bng,
                    Some(&ego),
                    CameraConfig {
                        requested_update_time: 0.01,
                        pos: (-0.3, 1.0, 2.0),
                        dir: (0.0, -1.0, 0.0),
                        field_of_view_y: 70.0,
                        near_far_planes: (0.1, 1000.0),
                        resolution: (W as u32, H as u32),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
                println!("Camera opened.");

                // Synchronized loop: apply controls → step sim → grab camera frame
                loop {
                    let tick_start = Instant::now();

                    // 1. Apply latest control input
                    let mut latest = None;
                    while let Ok(cmd) = control_rx.try_recv() {
                        latest = Some(cmd);
                    }
                    if let Some(ControlCmd::Drive {
                        steering,
                        throttle,
                        brake,
                        parkingbrake,
                    }) = latest
                    {
                        let _ = ego
                            .root()
                            .control(
                                Some(steering),
                                Some(throttle),
                                Some(brake),
                                Some(parkingbrake),
                                None,
                                None,
                            )
                            .await;
                    }

                    // 2. Advance simulation (renders the camera)
                    if let Err(e) = bng.control().step(STEPS_PER_FRAME, true).await {
                        eprintln!("Step error: {e}");
                        continue;
                    }

                    // 3. Grab the rendered frame (poll_raw = 1 round-trip vs ad-hoc's 3+)
                    match camera.poll_raw(&mut bng).await {
                        Ok(raw) => {
                            if let Some(rgba) = raw.colour.and_then(colour_to_rgba) {
                                let frame_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
                                let _ = frame_tx.try_send(Frame { rgba, frame_ms });
                            }
                        }
                        Err(e) => {
                            eprintln!("Camera poll error: {e}");
                        }
                    }
                }
            });
    });

    eframe::run_native(
        "BeamNG Manual Control",
        eframe::NativeOptions::default(),
        Box::new(|_cc| {
            Ok(Box::new(App {
                frame_rx,
                control_tx,
                texture: None,
                fps: 0.0,
                frame_ms: 0.0,
            }))
        }),
    )
}
