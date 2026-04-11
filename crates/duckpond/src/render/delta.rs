use crate::artifact::delta::*;
use crate::render::render_body;

impl Delta {
    /// Render the delta to canonical markdown.
    ///
    /// Entries are emitted in the order stored in the struct, which the parser
    /// guarantees is canonical (`=` → `-` → `~` → `@` → `+`).
    pub fn render(&self) -> String {
        let mut out = String::new();

        // H1 with marker
        out.push_str(&format!("# {} {}", self.marker.char(), self.title));

        // Optional summary
        if let Some(summary) = &self.summary {
            out.push_str(&format!("\n\n{summary}"));
        }

        // Description
        if !self.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&render_body(&self.description));
        }

        // H2 entries
        for entry in &self.entries {
            out.push_str("\n\n");
            render_h2_entry(&mut out, entry);
        }

        out.push('\n');
        out
    }
}

fn render_h2_entry(out: &mut String, entry: &DeltaEntry) {
    out.push_str(&format!("## {} {}", entry.marker.char(), entry.heading));

    // Rename: new name as first line
    if let Some(new_name) = &entry.rename_to {
        out.push_str(&format!("\n\n{new_name}"));
    }

    // Body
    if !entry.body.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&entry.body));
    }

    // Children
    match &entry.children {
        DeltaChildren::Operations(ops) => {
            for child in ops {
                out.push_str("\n\n");
                render_operation_child(out, child);
            }
        }
        DeltaChildren::Content(sections) => {
            for section in sections {
                out.push_str("\n\n");
                render_content_child(out, section);
            }
        }
    }
}

/// Render an operation child (under `@`): heading carries a marker.
fn render_operation_child(out: &mut String, entry: &DeltaChildEntry) {
    out.push_str(&format!("### {} {}", entry.marker.char(), entry.heading));

    if let Some(new_name) = &entry.rename_to {
        out.push_str(&format!("\n\n{new_name}"));
    }

    if !entry.body.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&entry.body));
    }
}

/// Render a content child (under `~` or `+`): heading has no marker.
fn render_content_child(out: &mut String, section: &crate::artifact::doc::Section) {
    let hashes = "#".repeat(section.level as usize);
    out.push_str(&format!("{hashes} {}", section.heading));

    if !section.body.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&section.body));
    }
}
