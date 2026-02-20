use beamng_proto::types::{Color, Float2, Vec3};
use beamng_proto::Result;

use crate::beamng::BeamNg;

/// API for drawing debug graphical objects in the simulator.
pub struct DebugApi<'a> {
    pub(crate) bng: &'a BeamNg,
}

fn vec3_to_value(v: Vec3) -> rmpv::Value {
    rmpv::Value::Array(vec![
        rmpv::Value::from(v.0),
        rmpv::Value::from(v.1),
        rmpv::Value::from(v.2),
    ])
}

fn color_to_value(c: Color) -> rmpv::Value {
    rmpv::Value::Array(vec![
        rmpv::Value::from(c.0),
        rmpv::Value::from(c.1),
        rmpv::Value::from(c.2),
        rmpv::Value::from(c.3),
    ])
}

impl DebugApi<'_> {
    /// Add debug spheres at the given coordinates.
    pub async fn add_spheres(
        &self,
        coordinates: &[Vec3],
        radii: &[f64],
        colors: &[Color],
        cling: bool,
        offset: f64,
    ) -> Result<Vec<i64>> {
        let coords: Vec<rmpv::Value> = coordinates.iter().map(|c| vec3_to_value(*c)).collect();
        let radii_val: Vec<rmpv::Value> = radii.iter().map(|r| rmpv::Value::from(*r)).collect();
        let colors_val: Vec<rmpv::Value> = colors.iter().map(|c| color_to_value(*c)).collect();

        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugSpheres",
                &[
                    ("coordinates", rmpv::Value::Array(coords)),
                    ("radii", rmpv::Value::Array(radii_val)),
                    ("colors", rmpv::Value::Array(colors_val)),
                    ("cling", rmpv::Value::from(cling)),
                    ("offset", rmpv::Value::from(offset)),
                ],
            )
            .await?;

        let ids = resp
            .get("sphereIDs")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        Ok(ids)
    }

    /// Remove debug spheres by their IDs.
    pub async fn remove_spheres(&self, sphere_ids: &[i64]) -> Result<()> {
        let ids: Vec<rmpv::Value> = sphere_ids.iter().map(|id| rmpv::Value::from(*id)).collect();
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("spheres")),
                    ("objIDs", rmpv::Value::Array(ids)),
                ],
            )
            .await
    }

    /// Add a debug polyline.
    pub async fn add_polyline(
        &self,
        coordinates: &[Vec3],
        color: Color,
        cling: bool,
        offset: f64,
    ) -> Result<i64> {
        let coords: Vec<rmpv::Value> = coordinates.iter().map(|c| vec3_to_value(*c)).collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugPolyline",
                &[
                    ("coordinates", rmpv::Value::Array(coords)),
                    ("color", color_to_value(color)),
                    ("cling", rmpv::Value::from(cling)),
                    ("offset", rmpv::Value::from(offset)),
                ],
            )
            .await?;

        resp.get("lineID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing lineID".into()))
    }

    /// Remove a debug polyline by ID.
    pub async fn remove_polyline(&self, line_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("polylines")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(line_id)]),
                    ),
                ],
            )
            .await
    }

    /// Add a debug cylinder between two circle centers.
    pub async fn add_cylinder(
        &self,
        circle_positions: &[Vec3; 2],
        radius: f64,
        color: Color,
    ) -> Result<i64> {
        let positions: Vec<rmpv::Value> =
            circle_positions.iter().map(|c| vec3_to_value(*c)).collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugCylinder",
                &[
                    ("circlePositions", rmpv::Value::Array(positions)),
                    ("radius", rmpv::Value::from(radius)),
                    ("color", color_to_value(color)),
                ],
            )
            .await?;

        resp.get("cylinderID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing cylinderID".into()))
    }

    /// Remove a debug cylinder by ID.
    pub async fn remove_cylinder(&self, cylinder_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("cylinders")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(cylinder_id)]),
                    ),
                ],
            )
            .await
    }

    /// Add a debug triangle.
    pub async fn add_triangle(
        &self,
        vertices: &[Vec3; 3],
        color: Color,
        cling: bool,
        offset: f64,
    ) -> Result<i64> {
        let verts: Vec<rmpv::Value> = vertices.iter().map(|v| vec3_to_value(*v)).collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugTriangle",
                &[
                    ("vertices", rmpv::Value::Array(verts)),
                    ("color", color_to_value(color)),
                    ("cling", rmpv::Value::from(cling)),
                    ("offset", rmpv::Value::from(offset)),
                ],
            )
            .await?;

        resp.get("triangleID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing triangleID".into()))
    }

    /// Remove a debug triangle by ID.
    pub async fn remove_triangle(&self, triangle_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("triangles")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(triangle_id)]),
                    ),
                ],
            )
            .await
    }

    /// Add a debug rectangle.
    pub async fn add_rectangle(
        &self,
        vertices: &[Vec3; 4],
        color: Color,
        cling: bool,
        offset: f64,
    ) -> Result<i64> {
        let verts: Vec<rmpv::Value> = vertices.iter().map(|v| vec3_to_value(*v)).collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugRectangle",
                &[
                    ("vertices", rmpv::Value::Array(verts)),
                    ("color", color_to_value(color)),
                    ("cling", rmpv::Value::from(cling)),
                    ("offset", rmpv::Value::from(offset)),
                ],
            )
            .await?;

        resp.get("rectangleID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing rectangleID".into()))
    }

    /// Remove a debug rectangle by ID.
    pub async fn remove_rectangle(&self, rectangle_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("rectangles")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(rectangle_id)]),
                    ),
                ],
            )
            .await
    }

    /// Add debug text at a position.
    pub async fn add_text(
        &self,
        origin: Vec3,
        content: &str,
        color: Color,
        cling: bool,
        offset: f64,
    ) -> Result<i64> {
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugText",
                &[
                    ("origin", vec3_to_value(origin)),
                    ("content", rmpv::Value::from(content)),
                    ("color", color_to_value(color)),
                    ("cling", rmpv::Value::from(cling)),
                    ("offset", rmpv::Value::from(offset)),
                ],
            )
            .await?;

        resp.get("textID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing textID".into()))
    }

    /// Remove debug text by ID.
    pub async fn remove_text(&self, text_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("text")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(text_id)]),
                    ),
                ],
            )
            .await
    }

    /// Add a debug square prism.
    pub async fn add_square_prism(
        &self,
        end_points: &[Vec3; 2],
        end_point_dims: &[Float2; 2],
        color: Color,
    ) -> Result<i64> {
        let points: Vec<rmpv::Value> = end_points.iter().map(|p| vec3_to_value(*p)).collect();
        let dims: Vec<rmpv::Value> = end_point_dims
            .iter()
            .map(|d| rmpv::Value::Array(vec![rmpv::Value::from(d.0), rmpv::Value::from(d.1)]))
            .collect();
        let resp = self
            .bng
            .conn()?
            .request(
                "AddDebugSquarePrism",
                &[
                    ("endPoints", rmpv::Value::Array(points)),
                    ("dims", rmpv::Value::Array(dims)),
                    ("color", color_to_value(color)),
                ],
            )
            .await?;

        resp.get("prismID")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| beamng_proto::BngError::ValueError("Missing prismID".into()))
    }

    /// Remove a debug square prism by ID.
    pub async fn remove_square_prism(&self, prism_id: i64) -> Result<()> {
        self.bng
            .conn()?
            .ack(
                "RemoveDebugObjects",
                "DebugObjectsRemoved",
                &[
                    ("objType", rmpv::Value::from("squarePrisms")),
                    (
                        "objIDs",
                        rmpv::Value::Array(vec![rmpv::Value::from(prism_id)]),
                    ),
                ],
            )
            .await
    }
}
