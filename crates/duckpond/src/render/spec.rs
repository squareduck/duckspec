use crate::artifact::spec::*;
use crate::render::render_body;

impl Spec {
    /// Render the spec to canonical markdown.
    pub fn render(&self) -> String {
        let mut out = String::new();

        // H1 + summary
        out.push_str(&format!("# {}\n\n{}", self.title, self.summary));

        // Description
        if !self.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&render_body(&self.description));
        }

        // Requirements
        for req in &self.requirements {
            out.push_str("\n\n");
            render_requirement(&mut out, req);
        }

        out.push('\n');
        out
    }
}

fn render_requirement(out: &mut String, req: &Requirement) {
    out.push_str(&format!("## Requirement: {}", req.name));

    // Prose
    if !req.prose.is_empty() {
        out.push_str("\n\n");
        out.push_str(&render_body(&req.prose));
    }

    // Requirement-level test marker
    if let Some(marker) = &req.test_marker {
        out.push_str("\n\n");
        render_test_marker(out, marker);
    }

    // Scenarios
    for scenario in &req.scenarios {
        out.push_str("\n\n");
        render_scenario(out, scenario);
    }
}

fn render_scenario(out: &mut String, scenario: &Scenario) {
    out.push_str(&format!("### Scenario: {}", scenario.name));

    // GWT clauses — reconstruct keywords from position in each array.
    let has_clauses =
        !scenario.givens.is_empty() || !scenario.whens.is_empty() || !scenario.thens.is_empty();

    if has_clauses {
        out.push('\n');

        for (i, clause) in scenario.givens.iter().enumerate() {
            let kw = if i == 0 { "GIVEN" } else { "AND" };
            out.push_str(&format!("\n- **{kw}** {}", clause.text));
        }

        for (i, clause) in scenario.whens.iter().enumerate() {
            let kw = if i == 0 { "WHEN" } else { "AND" };
            out.push_str(&format!("\n- **{kw}** {}", clause.text));
        }

        for (i, clause) in scenario.thens.iter().enumerate() {
            let kw = if i == 0 { "THEN" } else { "AND" };
            out.push_str(&format!("\n- **{kw}** {}", clause.text));
        }
    }

    // Scenario-level test marker
    if let Some(marker) = &scenario.test_marker {
        out.push_str("\n\n");
        render_test_marker(out, marker);
    }
}

fn render_test_marker(out: &mut String, marker: &TestMarker) {
    match &marker.kind {
        TestMarkerKind::Code { backlinks } => {
            out.push_str("> test: code");
            for link in backlinks {
                out.push_str(&format!("\n> - {}", link.path));
            }
        }
        TestMarkerKind::Manual { reason } => {
            out.push_str(&format!("> manual: {reason}"));
        }
        TestMarkerKind::Skip { reason } => {
            out.push_str(&format!("> skip: {reason}"));
        }
    }
}
