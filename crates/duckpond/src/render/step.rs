use crate::artifact::step::*;
use crate::render::render_body;

impl Step {
    /// Render the step to canonical markdown.
    pub fn render(&self) -> String {
        let mut out = String::new();

        // H1 + summary
        out.push_str(&format!("# {}\n\n{}", self.title, self.summary));

        // Description
        if !self.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&render_body(&self.description));
        }

        // Prerequisites
        if let Some(prereqs) = &self.prerequisites {
            out.push_str("\n\n## Prerequisites\n");
            for prereq in prereqs {
                let check = if prereq.checked { "x" } else { " " };
                let text = match &prereq.kind {
                    PrerequisiteKind::StepRef { slug } => format!("@step {slug}"),
                    PrerequisiteKind::Freeform { text } => text.clone(),
                };
                out.push_str(&format!("\n- [{check}] {text}"));
            }
        }

        // Context
        if let Some(ctx) = &self.context {
            out.push_str("\n\n## Context\n\n");
            out.push_str(&render_body(ctx));
        }

        // Tasks
        out.push_str("\n\n## Tasks\n");
        for (i, task) in self.tasks.iter().enumerate() {
            let num = i + 1;
            let check = if task.checked { "x" } else { " " };
            let text = render_task_content(&task.content);
            out.push_str(&format!("\n- [{check}] {num}. {text}"));
            for (j, sub) in task.subtasks.iter().enumerate() {
                let sub_num = format!("{}.{}", num, j + 1);
                let check = if sub.checked { "x" } else { " " };
                let text = render_task_content(&sub.content);
                out.push_str(&format!("\n  - [{check}] {sub_num} {text}"));
            }
        }

        // Outcomes
        if let Some(outcomes) = &self.outcomes {
            out.push_str("\n\n## Outcomes\n\n");
            out.push_str(&render_body(outcomes));
        }

        out.push('\n');
        out
    }
}

fn render_task_content(content: &TaskContent) -> String {
    match content {
        TaskContent::Freeform { text } => text.clone(),
        TaskContent::SpecRef {
            capability,
            requirement,
            scenario,
        } => format!("@spec {capability} {requirement}: {scenario}"),
    }
}
