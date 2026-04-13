//! Tree view with expand/collapse and selection.

use std::collections::HashSet;

use iced::widget::{button, column, row, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::data::TreeNode;
use crate::theme;

const ICON_FOLDER: &[u8] = include_bytes!("../../assets/icon_folder.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");

const ICON_SIZE: f32 = 14.0;

fn icon_for_leaf(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

/// Flat representation of a tree node for rendering.
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
    on_toggle: impl Fn(String) -> M + 'a,
    on_select: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let flat = flatten(nodes, expanded, 0);

    let mut col = column![].spacing(1.0);

    for node in flat {
        let indent = (node.depth as f32) * theme::SPACING_LG;
        let is_selected = selected.map_or(false, |s| s == node.id);

        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };

        let node_label = node.label.clone();
        let label = if node.has_children {
            let arrow = if node.is_expanded {
                "\u{25be}"
            } else {
                "\u{25b8}"
            };
            let icon = svg(svg::Handle::from_memory(ICON_FOLDER))
                .width(ICON_SIZE)
                .height(ICON_SIZE);
            row![
                text(arrow).size(theme::FONT_SM).color(theme::TEXT_MUTED),
                icon,
                text(node_label).size(theme::FONT_MD).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center)
        } else {
            let icon = svg(svg::Handle::from_memory(icon_for_leaf(&node_label)))
                .width(ICON_SIZE)
                .height(ICON_SIZE);
            row![
                Space::new().width(theme::FONT_SM),
                icon,
                text(node_label).size(theme::FONT_MD).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center)
        };

        let btn_content = row![Space::new().width(indent), label].align_y(iced::Center);

        let msg = if node.has_children {
            on_toggle(node.id.clone())
        } else {
            on_select(node.id.clone())
        };

        let btn = button(btn_content)
            .on_press(msg)
            .width(Length::Fill)
            .padding([2.0, theme::SPACING_SM])
            .style(style);

        col = col.push(btn);
    }

    col.into()
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
