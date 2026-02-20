use beamng_rs::sensors::{Camera, CameraConfig};
use beamng_rs::vehicle::{Vehicle, VehicleOptions};
use beamng_rs::{BeamNg, Scenario};

#[tokio::main]
async fn main() -> beamng_proto::Result<()> {
    tracing_subscriber::fmt::init();

    let bng = BeamNg::new("192.168.1.85", 5555).connect().await?;
    println!("Connected to BeamNG.tech!");

    // Return to main menu to get a clean state, ignore errors if already there
    let _ = bng.control().return_to_main_menu().await;

    // Clean up any leftover scenario from previous runs
    let _ = Scenario::delete(
        &bng,
        "/levels/italy/scenarios/camera_streaming/camera_streaming.json",
    )
    .await;

    // Create scenario
    let mut scenario = Scenario::new("italy", "camera_streaming");
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
    scenario.make(&bng).await?;
    println!("Scenario created.");

    // Configure and load (connects vehicles during load, matching Python SDK)
    let mut ego = Vehicle::new("ego", "etk800");
    bng.settings().set_deterministic(Some(60), None).await?;
    bng.scenario().load_scenario(&scenario, true, &mut [&mut ego]).await?;
    bng.scenario().start(false).await?;
    println!("Scenario started.");

    bng.control().pause().await?;
    ego.ai().set_mode("traffic").await?;

    // Open the camera sensor with shared-memory streaming
    let camera = Camera::open(
        "camera1",
        &bng,
        Some(&ego),
        CameraConfig {
            requested_update_time: 0.01,
            is_using_shared_memory: true,
            is_streaming: true,
            pos: (-0.3, 1.0, 2.0),
            dir: (0.0, -1.0, 0.0),
            field_of_view_y: 70.0,
            near_far_planes: (0.1, 1000.0),
            resolution: (1024, 1024),
            is_render_annotations: true,
            is_render_instance: true,
            is_render_depth: true,
            ..Default::default()
        },
    )
    .await?;
    println!("Camera opened.");

    for i in 0..41 {
        bng.control().step(10, true).await?;

        // Read raw bytes directly from shared memory — the fastest path
        let raw = camera.stream_raw()?;

        if i % 10 == 0 {
            if let Some(ref colour) = raw.colour {
                println!("Frame {i}: colour={} bytes", colour.len());
            }
            if let Some(ref annotation) = raw.annotation {
                println!("Frame {i}: annotation={} bytes", annotation.len());
            }
            if let Some(ref depth) = raw.depth {
                println!("Frame {i}: depth={} bytes", depth.len());
            }
        }
    }

    camera.close().await?;
    bng.control().resume().await?;
    println!("Done!");

    Ok(())
}
