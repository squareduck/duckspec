//! Tree view with expand/collapse and selection.

use std::collections::HashSet;

use iced::Element;
use iced::widget::{Space, row};

use crate::data::TreeNode;
use crate::theme;
use crate::widget::collapsible;
use crate::widget::list_view::{self, ListRow};

const ICON_FOLDER: &[u8] = include_bytes!("../../assets/icon_folder.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");

fn icon_for_leaf(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

struct FlatNode {
    id: String,
    label: String,
    depth: usize,
    has_children: bool,
    is_expanded: bool,
}

pub fn view<'a, M: Clone + 'a>(
    nodes: &[TreeNode],
    expanded: &HashSet<String>,
    selected: Option<&str>,
    error_ids: &HashSet<String>,
    on_toggle: impl Fn(String) -> M + 'a,
    on_select: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let rows: Vec<ListRow<'a, M>> = flatten(nodes, expanded, 0)
        .into_iter()
        .map(|node| {
            let is_selected = selected.is_some_and(|s| s == node.id);
            let has_error = error_ids.contains(&node.id);

            let (leading, icon_bytes, on_press) = if node.has_children {
                let leading = collapsible::chevron(node.is_expanded);
                (leading, ICON_FOLDER, on_toggle(node.id.clone()))
            } else {
                let leading: Element<'a, M> = row![Space::new().width(theme::font_sm())].into();
                (
                    leading,
                    icon_for_leaf(&node.label),
                    on_select(node.id.clone()),
                )
            };

            ListRow::new(node.label)
                .leading(leading)
                .icon(icon_bytes)
                .indent(node.depth)
                .selected(is_selected)
                .errored(has_error)
                .on_press(on_press)
        })
        .collect();

    list_view::view(rows, None)
}

fn flatten(nodes: &[TreeNode], expanded: &HashSet<String>, depth: usize) -> Vec<FlatNode> {
    let mut result = vec![];
    for node in nodes {
        let is_expanded = expanded.contains(&node.id);
        result.push(FlatNode {
            id: node.id.clone(),
            label: node.label.clone(),
            depth,
            has_children: !node.children.is_empty(),
            is_expanded,
        });
        if is_expanded {
            result.extend(flatten(&node.children, expanded, depth + 1));
        }
    }
    result
}
