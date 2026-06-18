# Change scope facts and the phase ladder

Widen the existing next-stage ladder into a `ChangeScopeFacts` source and carry it onto
each session beside `obvious_command`.

## Context

The ladder lives in `obvious_command_from_artifacts` (`area/change.rs`) and is pushed onto
every session as `ax.obvious_command` by `refresh_obvious_command` in the same file. Step
completion comes from `ChangeData.steps` (`Vec<StepInfo>`); each `StepInfo.completion` is
`StepCompletion::{NoTasks,
Partial(done, total), Done}`. `Done` does not carry its task
total, so report step-level progress (`steps_done / step_count`) plus the active `Partial`
step's `(done, total)` tally — do not attempt a full task aggregate.

The lifecycle-reflection scenarios are unit-tested here against `change_scope_facts`: the
orientation surfaces `next_command` and the progress counts verbatim, so testing the
derivation is the falsifiable contract for those scenarios.

## Tasks

- [x] 1. Define `pub struct ChangeScopeFacts` in `area/change.rs` with fields
         `phase: &'static str`, `steps_done: usize`, `step_count: usize`,
         `active_step_tasks: Option<(usize, usize)>`,
         `next_command:
                       Option<String>`; derive `Clone`

- [x] 2. Add
         `pub fn change_scope_facts(name: &str, project: &ProjectData) ->
                       Option<ChangeScopeFacts>`
         that walks the existing artifact/step ladder and fills every field, including a
         `phase` label per lifecycle rung

- [x] 3. Reduce `obvious_command_from_artifacts` to a thin caller:
         `change_scope_facts(name, project).and_then(|f| f.next_command)`, so the
         placeholder command keeps its current behavior

- [x] 4. Add `pub scope_facts: Option<ChangeScopeFacts>` to `AgentSession`
         (`area/interaction.rs`), defaulting to `None` at every construction site, and
         populate it in `refresh_obvious_command` beside `obvious_command` (computed once
         per change scope, `None` for non-change scopes)

- [x] 5. @spec session/scope Lifecycle reflection: A change with unfinished steps reports remaining work and the apply next-stage

- [x] 6. @spec session/scope Lifecycle reflection: A change with all steps complete reports completion and the archive next-stage

- [x] 7. @spec session/scope Lifecycle reflection: A change with only a proposal reports the design next-stage
