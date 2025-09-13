use crate::nav::keys::{child_key, menu_key};
use crate::ui::{is_header, AppState, FlatNode};

pub fn flatten_nodes(state: &AppState) -> Vec<FlatNode> {
    fn append_children(out: &mut Vec<FlatNode>, state: &AppState, parent_key: &str, depth: usize) {
        if let Some(children) = state.children.get(parent_key) {
            for (ci, val) in children.iter().enumerate() {
                let key = child_key(parent_key, val, ci);
                out.push(FlatNode::Child {
                    key: key.clone(),
                    depth,
                    val: val.clone(),
                });
                // Recurse into children when this node is expanded, regardless of how
                // the children are provided (static inline, lazy or autoload).
                if state.expanded.contains(&key) {
                    append_children(out, state, &key, depth + 1);
                }
            }
        }
    }

    let mut out = Vec::new();
    for (i, mi) in state.config.menu.iter().enumerate() {
        if is_header(mi) {
            out.push(FlatNode::Header { idx: i, depth: 0 });
            continue;
        }
        out.push(FlatNode::Menu { idx: i, depth: 0 });
        let key = menu_key(mi);
        if state.expanded.contains(&key) {
            append_children(&mut out, state, &key, 1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppConfig, MenuItem};
    use serde_json::json;

    fn make_state() -> AppState {
        let mut state = AppState::default();
        let mi_header = MenuItem {
            id: "hdr".into(),
            title: "Header".into(),
            widget: Some("header".into()),
            ..Default::default()
        };
        let mi_lazy = MenuItem {
            id: "m1".into(),
            title: "Lazy".into(),
            widget: Some("lazy_items".into()),
            command: Some("example-app list-items".into()),
            unwrap: Some("data.items".into()),
            initial_text: Some("Enter to load".into()),
            auto_expand: Some(true),
            expand_on_enter: Some(false),
            ..Default::default()
        };
        state.config = AppConfig {
            header: Some("Test".into()),
            menu: vec![mi_header.clone(), mi_lazy.clone()],
            ..Default::default()
        };

        // Simulate loaded children for the lazy menu
        let key = menu_key(&mi_lazy);
        state.expanded.insert(key.clone());
        state.children.insert(
            key.clone(),
            vec![json!({"id":"c1","title":"Child","widget":"lazy_items"})],
        );

        let child_k = child_key(&key, &state.children.get(&key).unwrap()[0], 0);
        state.expanded.insert(child_k.clone());
        state
            .children
            .insert(child_k, vec![json!({"id":"gc1","title":"Grandchild"})]);
        state
    }

    #[test]
    fn flattens_headers_menus_and_children() {
        let state = make_state();
        let nodes = flatten_nodes(&state);
        // Expect: header, menu (lazy), child, grandchild
        assert!(nodes.len() >= 4);
        assert!(matches!(nodes[0], FlatNode::Header { .. }));
        assert!(matches!(nodes[1], FlatNode::Menu { .. }));
        assert!(matches!(nodes[2], FlatNode::Child { .. }));
        assert!(matches!(nodes[3], FlatNode::Child { .. }));
    }
}
