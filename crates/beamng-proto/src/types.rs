use std::collections::HashMap;

/// A 3D vector.
pub type Vec3 = (f64, f64, f64);

/// A quaternion (x, y, z, w).
pub type Quat = (f64, f64, f64, f64);

/// An RGBA color with components in [0.0, 1.0].
pub type Color = (f64, f64, f64, f64);

/// A 2D float pair.
pub type Float2 = (f64, f64);

/// A 2D integer pair (e.g. resolution).
pub type Int2 = (u32, u32);

/// A generic string-keyed dictionary mirroring Python's `StrDict`.
pub type StrDict = HashMap<String, rmpv::Value>;

/// Extract a string from a [`rmpv::Value`], handling both `String` and `Binary` types.
///
/// BeamNG.tech sends some string values as msgpack `bin` (due to Python's `use_bin_type=True`),
/// so we need to handle both encodings.
pub fn value_to_string(val: &rmpv::Value) -> Option<String> {
    match val {
        rmpv::Value::String(s) => s.as_str().map(|s| s.to_string()),
        rmpv::Value::Binary(b) => std::str::from_utf8(b).ok().map(|s| s.to_string()),
        _ => None,
    }
}

/// Extract a string slice from a [`rmpv::Value`], only works for `String` variant.
/// For values that might be `Binary`, use [`value_to_string`] instead.
pub fn value_as_str(val: &rmpv::Value) -> Option<&str> {
    match val {
        rmpv::Value::String(s) => s.as_str(),
        rmpv::Value::Binary(b) => std::str::from_utf8(b).ok(),
        _ => None,
    }
}

/// Extract a key string from a [`rmpv::Value`] map key (handles both `String` and `Binary`).
fn key_to_string(val: rmpv::Value) -> Option<String> {
    match val {
        rmpv::Value::String(s) => s.into_str(),
        rmpv::Value::Binary(b) => String::from_utf8(b).ok(),
        _ => None,
    }
}

/// Extract a `u64` from a [`rmpv::Value`], handling integer and float types.
///
/// BeamNG.tech sends `_id` as `f64` in some cases.
pub fn value_as_u64(val: &rmpv::Value) -> Option<u64> {
    val.as_u64()
        .or_else(|| val.as_i64().map(|i| i as u64))
        .or_else(|| val.as_f64().map(|f| f as u64))
}

/// Convert a [`rmpv::Value`] to a [`StrDict`], returning `None` if it's not a map.
///
/// Handles both `String` and `Binary` keys (BeamNG uses a mix).
pub fn value_to_str_dict(val: rmpv::Value) -> Option<StrDict> {
    match val {
        rmpv::Value::Map(pairs) => {
            let mut map = HashMap::with_capacity(pairs.len());
            for (k, v) in pairs {
                let key = key_to_string(k)?;
                map.insert(key, v);
            }
            Some(map)
        }
        _ => None,
    }
}

/// Helper to extract a bool from a [`rmpv::Value`].
pub fn value_as_bool(val: &rmpv::Value) -> Option<bool> {
    val.as_bool()
}

/// Helper to extract an f64 from a [`rmpv::Value`].
pub fn value_as_f64(val: &rmpv::Value) -> Option<f64> {
    val.as_f64()
}

/// Compute a 3x3 rotation matrix (row-major, 9 elements) from a quaternion (x, y, z, w).
pub fn quat_to_rotation_matrix(q: Quat) -> [f64; 9] {
    let (x, y, z, w) = q;
    let norm = (x * x + y * y + z * z + w * w).sqrt();
    let (x, y, z, w) = if (norm - 1.0).abs() > f64::EPSILON {
        (x / norm, y / norm, z / norm, w / norm)
    } else {
        (x, y, z, w)
    };
    [
        1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w),
        2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w),
        2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y),
    ]
}

/// Format a quaternion as a rotation matrix string `"[0.123, 0.456, ...]"` for prefab JSON.
pub fn quat_as_rotation_matrix_str(q: Quat) -> String {
    let mat = quat_to_rotation_matrix(q);
    let parts: Vec<String> = mat.iter().map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

/// Build a [`rmpv::Value::Map`] from key-value pairs conveniently.
///
/// ```
/// use beamng_proto::types::str_dict;
/// let map = str_dict([
///     ("type", rmpv::Value::from("Hello")),
///     ("protocolVersion", rmpv::Value::from("v1.26")),
/// ]);
/// ```
pub fn str_dict<const N: usize>(pairs: [(&str, rmpv::Value); N]) -> rmpv::Value {
    let vec: Vec<(rmpv::Value, rmpv::Value)> = pairs
        .into_iter()
        .map(|(k, v)| (rmpv::Value::from(k), v))
        .collect();
    rmpv::Value::Map(vec)
}
