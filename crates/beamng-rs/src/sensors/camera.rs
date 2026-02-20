use beamng_proto::types::{Float2, Int2, Vec3};
use beamng_proto::{BngError, Result};
use shared_memory::{Shmem, ShmemConf};
use tracing::info;

use crate::beamng::BeamNg;
use crate::vehicle::Vehicle;

/// Configuration for a [`Camera`] sensor.
///
/// All fields have defaults matching the Python SDK.
#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub requested_update_time: f64,
    pub update_priority: f64,
    pub resolution: Int2,
    pub field_of_view_y: f64,
    pub near_far_planes: Float2,
    pub pos: Vec3,
    pub dir: Vec3,
    pub up: Vec3,
    pub is_using_shared_memory: bool,
    pub is_streaming: bool,
    pub is_render_colours: bool,
    pub is_render_annotations: bool,
    pub is_render_instance: bool,
    pub is_render_depth: bool,
    pub is_visualised: bool,
    pub is_static: bool,
    pub is_snapping_desired: bool,
    pub is_force_inside_triangle: bool,
    pub is_dir_world_space: bool,
    pub integer_depth: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            requested_update_time: 0.0,
            update_priority: 0.0,
            resolution: (512, 512),
            field_of_view_y: 70.0,
            near_far_planes: (0.05, 100.0),
            pos: (0.0, 0.0, 0.0),
            dir: (0.0, -1.0, 0.0),
            up: (0.0, 0.0, 1.0),
            is_using_shared_memory: false,
            is_streaming: false,
            is_render_colours: true,
            is_render_annotations: false,
            is_render_instance: false,
            is_render_depth: false,
            is_visualised: true,
            is_static: false,
            is_snapping_desired: false,
            is_force_inside_triangle: false,
            is_dir_world_space: false,
            integer_depth: false,
        }
    }
}

/// Raw image data from a camera reading.
pub struct CameraRawReadings {
    pub colour: Option<Vec<u8>>,
    pub annotation: Option<Vec<u8>>,
    pub depth: Option<Vec<u8>>,
}

/// Wraps an OS shared memory segment.
struct ShmemBuffer {
    shmem: Shmem,
    size: usize,
}

impl ShmemBuffer {
    fn create(size: usize) -> Result<Self> {
        let shmem = ShmemConf::new()
            .size(size)
            .create()
            .map_err(|e| BngError::Io(std::io::Error::other(format!("shared memory create: {e}"))))?;
        Ok(Self { shmem, size })
    }

    fn name(&self) -> &str {
        self.shmem.get_os_id()
    }

    fn read(&self) -> Vec<u8> {
        let ptr = self.shmem.as_ptr();
        let slice = unsafe { std::slice::from_raw_parts(ptr, self.size) };
        slice.to_vec()
    }
}

/// A camera sensor attached to the simulator (GE-level), optionally tracking a vehicle.
///
/// Uses shared memory for high-performance image streaming. The camera communicates
/// through the main BeamNG connection, not through a per-vehicle connection.
///
/// # Example
/// ```no_run
/// # async fn example() -> beamng_proto::Result<()> {
/// use beamng_rs::BeamNg;
/// use beamng_rs::sensors::{Camera, CameraConfig};
///
/// let bng = BeamNg::new("localhost", 25252).connect().await?;
/// let camera = Camera::open("cam1", &bng, None, CameraConfig {
///     is_using_shared_memory: true,
///     is_streaming: true,
///     resolution: (1024, 1024),
///     ..Default::default()
/// }).await?;
/// let raw = camera.stream_raw()?;
/// camera.close().await?;
/// # Ok(())
/// # }
/// ```
pub struct Camera<'a> {
    name: String,
    bng: &'a BeamNg,
    config: CameraConfig,
    colour_shmem: Option<ShmemBuffer>,
    annotation_shmem: Option<ShmemBuffer>,
    depth_shmem: Option<ShmemBuffer>,
}

impl<'a> Camera<'a> {
    /// Open a camera sensor in the simulator.
    ///
    /// Creates shared memory buffers (if configured) and sends `OpenCamera` to the simulator.
    pub async fn open(
        name: impl Into<String>,
        bng: &'a BeamNg,
        vehicle: Option<&Vehicle>,
        config: CameraConfig,
    ) -> Result<Camera<'a>> {
        let name = name.into();
        let buf_size = (config.resolution.0 * config.resolution.1 * 4) as usize;

        let colour_shmem = if config.is_using_shared_memory && config.is_render_colours {
            Some(ShmemBuffer::create(buf_size)?)
        } else {
            None
        };
        let annotation_shmem = if config.is_using_shared_memory
            && (config.is_render_annotations || config.is_render_instance)
        {
            Some(ShmemBuffer::create(buf_size)?)
        } else {
            None
        };
        let depth_shmem = if config.is_using_shared_memory && config.is_render_depth {
            Some(ShmemBuffer::create(buf_size)?)
        } else {
            None
        };

        // Build the OpenCamera request fields
        let vid_val: rmpv::Value = match vehicle {
            Some(v) => rmpv::Value::from(v.vid.as_str()),
            None => rmpv::Value::from(0),
        };

        let colour_shmem_name: rmpv::Value = match &colour_shmem {
            Some(s) => rmpv::Value::from(s.name()),
            None => rmpv::Value::Nil,
        };
        let colour_shmem_size: rmpv::Value = match &colour_shmem {
            Some(s) => rmpv::Value::from(s.size as i64),
            None => rmpv::Value::from(-1),
        };
        let annotation_shmem_name: rmpv::Value = match &annotation_shmem {
            Some(s) => rmpv::Value::from(s.name()),
            None => rmpv::Value::Nil,
        };
        let annotation_shmem_size: rmpv::Value = match &annotation_shmem {
            Some(s) => rmpv::Value::from(s.size as i64),
            None => rmpv::Value::from(-1),
        };
        let depth_shmem_name: rmpv::Value = match &depth_shmem {
            Some(s) => rmpv::Value::from(s.name()),
            None => rmpv::Value::Nil,
        };
        let depth_shmem_size: rmpv::Value = match &depth_shmem {
            Some(s) => rmpv::Value::from(s.size as i64),
            None => rmpv::Value::from(-1),
        };

        let fields: Vec<(&str, rmpv::Value)> = vec![
            ("vid", vid_val),
            ("name", rmpv::Value::from(name.as_str())),
            ("updateTime", rmpv::Value::from(config.requested_update_time)),
            ("priority", rmpv::Value::from(config.update_priority)),
            (
                "size",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.resolution.0),
                    rmpv::Value::from(config.resolution.1),
                ]),
            ),
            ("fovY", rmpv::Value::from(config.field_of_view_y)),
            (
                "nearFarPlanes",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.near_far_planes.0),
                    rmpv::Value::from(config.near_far_planes.1),
                ]),
            ),
            (
                "pos",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.pos.0),
                    rmpv::Value::from(config.pos.1),
                    rmpv::Value::from(config.pos.2),
                ]),
            ),
            (
                "dir",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.dir.0),
                    rmpv::Value::from(config.dir.1),
                    rmpv::Value::from(config.dir.2),
                ]),
            ),
            (
                "up",
                rmpv::Value::Array(vec![
                    rmpv::Value::from(config.up.0),
                    rmpv::Value::from(config.up.1),
                    rmpv::Value::from(config.up.2),
                ]),
            ),
            ("useSharedMemory", rmpv::Value::from(config.is_using_shared_memory)),
            ("colourShmemName", colour_shmem_name),
            ("colourShmemSize", colour_shmem_size),
            ("annotationShmemName", annotation_shmem_name),
            ("annotationShmemSize", annotation_shmem_size),
            ("depthShmemName", depth_shmem_name),
            ("depthShmemSize", depth_shmem_size),
            ("renderColours", rmpv::Value::from(config.is_render_colours)),
            ("renderAnnotations", rmpv::Value::from(config.is_render_annotations)),
            ("renderInstance", rmpv::Value::from(config.is_render_instance)),
            ("renderDepth", rmpv::Value::from(config.is_render_depth)),
            ("isVisualised", rmpv::Value::from(config.is_visualised)),
            ("isStreaming", rmpv::Value::from(config.is_streaming)),
            ("isStatic", rmpv::Value::from(config.is_static)),
            ("isSnappingDesired", rmpv::Value::from(config.is_snapping_desired)),
            ("isForceInsideTriangle", rmpv::Value::from(config.is_force_inside_triangle)),
            ("isDirWorldSpace", rmpv::Value::from(config.is_dir_world_space)),
            ("integerDepth", rmpv::Value::from(config.integer_depth)),
        ];

        bng.conn()?
            .ack("OpenCamera", "OpenedCamera", &fields)
            .await?;

        info!("Opened Camera: \"{}\"", name);

        Ok(Camera {
            name,
            bng,
            config,
            colour_shmem,
            annotation_shmem,
            depth_shmem,
        })
    }

    /// Read raw image data directly from shared memory without sending any request.
    ///
    /// This is the fastest path — no network round-trip. Requires the camera to have been
    /// created with `is_streaming: true` and `is_using_shared_memory: true`.
    pub fn stream_raw(&self) -> Result<CameraRawReadings> {
        if !self.config.is_streaming {
            return Err(BngError::ValueError(
                "This camera was not created with is_streaming=true. Stream not available.".into(),
            ));
        }
        if !self.config.is_using_shared_memory {
            return Err(BngError::ValueError(
                "This camera was not created with is_using_shared_memory=true.".into(),
            ));
        }

        Ok(CameraRawReadings {
            colour: self.colour_shmem.as_ref().map(|s| s.read()),
            annotation: self.annotation_shmem.as_ref().map(|s| s.read()),
            depth: self.depth_shmem.as_ref().map(|s| s.read()),
        })
    }

    /// Poll the simulator for the latest camera reading, then read from shared memory.
    ///
    /// Sends a `PollCamera` message and waits for the response before reading shared memory.
    pub async fn poll_raw(&self) -> Result<CameraRawReadings> {
        let conn = self.bng.conn()?;
        conn.request(
            "PollCamera",
            &[
                ("name", rmpv::Value::from(self.name.as_str())),
                (
                    "isUsingSharedMemory",
                    rmpv::Value::from(self.config.is_using_shared_memory),
                ),
            ],
        )
        .await?;

        Ok(CameraRawReadings {
            colour: self.colour_shmem.as_ref().map(|s| s.read()),
            annotation: self.annotation_shmem.as_ref().map(|s| s.read()),
            depth: self.depth_shmem.as_ref().map(|s| s.read()),
        })
    }

    /// Close the camera sensor and release shared memory.
    pub async fn close(self) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "CloseCamera",
                "ClosedCamera",
                &[("name", rmpv::Value::from(self.name.as_str()))],
            )
            .await?;
        info!("Closed Camera: \"{}\"", self.name);
        // Shared memory buffers are dropped here automatically
        Ok(())
    }

    /// Get the camera name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the camera configuration.
    pub fn config(&self) -> &CameraConfig {
        &self.config
    }
}
