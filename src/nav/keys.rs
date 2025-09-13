use crate::model::MenuItem;
use serde_json::Value as JsonValue;

pub fn menu_key(mi: &MenuItem) -> String {
    format!("menu:{}", mi.id)
}

pub fn child_key(parent_key: &str, v: &JsonValue, idx: usize) -> String {
    if let Some(id) = v.get("id").and_then(|s| s.as_str()) {
        format!("{parent_key}/{id}")
    } else {
        format!("{parent_key}/#{idx}")
    }
}
