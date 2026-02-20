use beamng_rs::sensors::{Camera, CameraConfig};
use beamng_rs::vehicle::{Vehicle, VehicleOptions};
use beamng_rs::{BeamNg, Scenario};
use eframe::egui;
use tokio::sync::mpsc;

const W: usize = 1280;
const H: usize = 720;

enum ControlCmd {
    Drive {
        steering: f64,
        throttle: f64,
        brake: f64,
        parkingbrake: f64,
    },
}

struct App {
    frame_rx: mpsc::Receiver<Vec<u8>>,
    control_tx: mpsc::Sender<ControlCmd>,
    texture: Option<egui::TextureHandle>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain channel, keep only the latest frame
        let mut latest = None;
        while let Ok(f) = self.frame_rx.try_recv() {
            latest = Some(f);
        }

        // Upload to GPU texture
        if let Some(bytes) = latest {
            let img = egui::ColorImage::from_rgba_unmultiplied([W, H], &bytes);
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

        // Display camera feed
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                ui.image(tex);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Connecting to BeamNG...");
                });
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    let (frame_tx, frame_rx) = mpsc::channel(2);
    let (control_tx, mut control_rx) = mpsc::channel(16);

    // Background thread: tokio runtime with BeamNG connection
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let bng = BeamNg::new("192.168.1.85", 5555).connect().await.unwrap();
                println!("Connected to BeamNG.tech!");

                let _ = bng.control().return_to_main_menu().await;
                let _ = Scenario::delete(
                    &bng,
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
                scenario.make(&bng).await.unwrap();
                println!("Scenario created.");

                let mut ego = Vehicle::new("ego", "etk800");
                bng.scenario()
                    .load_scenario(&scenario, true, &mut [&mut ego])
                    .await
                    .unwrap();
                bng.scenario().start(false).await.unwrap();
                println!("Scenario started (real-time mode).");

                let camera = Camera::open(
                    "camera1",
                    &bng,
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
                println!("Camera opened (network polling).");

                // Spawn a separate task for vehicle controls so they run at ~60fps
                // independent of camera round-trip time
                tokio::spawn(async move {
                    loop {
                        // Drain channel, apply latest command
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
                            if let Err(e) = ego
                                .root()
                                .control(
                                    Some(steering),
                                    Some(throttle),
                                    Some(brake),
                                    Some(parkingbrake),
                                    None,
                                    None,
                                )
                                .await
                            {
                                eprintln!("Control error: {e}");
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                    }
                });

                // Camera loop (decoupled from controls)
                loop {
                    match camera.ad_hoc_poll_raw().await {
                        Ok(raw) => {
                            if let Some(colour) = raw.colour {
                                let expected_rgb = W * H * 3;
                                let expected_rgba = W * H * 4;
                                if colour.len() == expected_rgb {
                                    // Network: RGB → RGBA
                                    let mut buf = Vec::with_capacity(expected_rgba);
                                    for px in colour.chunks_exact(3) {
                                        buf.extend_from_slice(px);
                                        buf.push(255);
                                    }
                                    let _ = frame_tx.try_send(buf);
                                } else if colour.len() == expected_rgba {
                                    // RGBA with alpha=0
                                    let mut buf = colour;
                                    for px in buf.chunks_exact_mut(4) {
                                        px[3] = 255;
                                    }
                                    let _ = frame_tx.try_send(buf);
                                }
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
            }))
        }),
    )
}
