//! The model editor's layout: an opaque JSON object stored with a model (ADR 0010).
//!
//! The backend never interprets it. It only checks the size and shape, and replaces element
//! ids inside it when a model is copied, so the copy's layout points at the copy's elements.

use std::collections::HashMap;

use serde_json::{Map, Value};
use uuid::Uuid;

/// Largest accepted layout, serialized.
pub const MAX_LAYOUT_BYTES: usize = 256 * 1024;

/// Why a layout was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// Not a JSON object.
    #[error("must be a JSON object")]
    NotAnObject,
    /// Over [`MAX_LAYOUT_BYTES`].
    #[error("must be at most {} KB", MAX_LAYOUT_BYTES / 1024)]
    TooLarge,
}

/// Checks a layout before it is stored.
pub fn check_layout(layout: &Value) -> Result<(), LayoutError> {
    if !layout.is_object() {
        return Err(LayoutError::NotAnObject);
    }
    // Serializing a `Value` cannot fail.
    let size = serde_json::to_vec(layout).map_or(usize::MAX, |b| b.len());
    if size > MAX_LAYOUT_BYTES {
        return Err(LayoutError::TooLarge);
    }
    Ok(())
}

/// Replaces every object key and string value equal to an old id with its new id.
pub fn remap_layout_ids(layout: &Value, ids: &HashMap<Uuid, Uuid>) -> Value {
    let swap = |s: &str| {
        Uuid::parse_str(s)
            .ok()
            .and_then(|id| ids.get(&id))
            .map_or_else(|| s.to_owned(), |new| new.to_string())
    };
    match layout {
        Value::String(s) => Value::String(swap(s)),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| remap_layout_ids(v, ids)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (swap(k), remap_layout_ids(v, ids)))
                .collect::<Map<_, _>>(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_objects_only() {
        assert_eq!(check_layout(&json!({ "v": 1 })), Ok(()));
        assert_eq!(check_layout(&json!([1])), Err(LayoutError::NotAnObject));
        assert_eq!(check_layout(&json!("x")), Err(LayoutError::NotAnObject));
    }

    #[test]
    fn rejects_oversized() {
        let big = json!({ "pad": "x".repeat(MAX_LAYOUT_BYTES) });
        assert_eq!(check_layout(&big), Err(LayoutError::TooLarge));
    }

    #[test]
    fn remaps_keys_and_values_anywhere() {
        let (a, b, keep) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (a2, b2) = (Uuid::new_v4(), Uuid::new_v4());
        let ids = HashMap::from([(a, a2), (b, b2)]);
        let layout = json!({
            "v": 1,
            "states": { a.to_string(): { "shape": "diamond" }, keep.to_string(): {} },
            "groups": [ { "members": [b.to_string()], "label": "not an id" } ],
        });
        assert_eq!(
            remap_layout_ids(&layout, &ids),
            json!({
                "v": 1,
                "states": { a2.to_string(): { "shape": "diamond" }, keep.to_string(): {} },
                "groups": [ { "members": [b2.to_string()], "label": "not an id" } ],
            })
        );
    }
}
