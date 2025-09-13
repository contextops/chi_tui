use super::*;
use serde_json::json;

#[test]
fn get_by_path_traverses_nested_objects() {
    let v = json!({
        "data": {
            "items": [1, 2, 3],
            "meta": {"page": 1}
        }
    });
    assert_eq!(
        get_by_path(&v, "data.items")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        get_by_path(&v, "data.meta.page").unwrap().as_i64().unwrap(),
        1
    );
    assert!(get_by_path(&v, "data.missing").is_none());
    assert!(get_by_path(&v, "").is_none());
}

#[test]
fn parse_options_supports_multiple_unwrap_patterns() {
    let v = json!({
        "data": {
            "items": [
                {"id": "a", "title": "Alpha"},
                {"id": "b", "title": "Bravo"}
            ],
            "tags": ["urgent", "normal"],
        }
    });
    let pairs = parse_options_from_json(&v, Some("data.items[].id/title"));
    assert_eq!(pairs[0].0, "Alpha");
    assert_eq!(pairs[0].1, "a");
    let pairs2 = parse_options_from_json(&v, Some("data.tags"));
    assert_eq!(pairs2[1].0, "normal");
    assert_eq!(pairs2[1].1, "normal");
}
