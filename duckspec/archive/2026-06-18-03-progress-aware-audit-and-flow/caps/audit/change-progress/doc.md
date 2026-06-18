# Change audit progress

How a change-scoped audit reports the `test:code` scenarios a change introduces that do
not yet have a source backlink. Instead of treating every unlinked scenario as a failure,
the audit reads the change's step tasks to tell work that is simply not done yet from work
that was marked done but left unlinked.

A change is implemented one step at a time, and a scenario is linked to code by a `@spec`
comment on the test that covers it. Between specifying a change and finishing its last
step, most scenarios have no backlink yet — that is the normal state of a change in
progress, not a defect. This capability makes the scoped audit say so.

## Classification

The audit looks only at `test:code` scenarios the change introduces that have no resolving
source backlink. Each one is placed in a category by the step tasks that reference it:

```text
| Category | Condition                                   | Counts as error? |
| -------- | ------------------------------------------- | ---------------- |
| linked   | a source backlink resolves                  | no               |
| pending  | no referencing step task is checked         | no               |
| error    | at least one referencing step task is checked | yes            |
```

A scenario is "claimed" — and therefore an error if still unlinked — as soon as **any**
step task referencing it is checked off. Checking the task that says "implement this
scenario as a test" is a claim that the work is done, so a missing backlink at that point
is a real defect rather than pending work. A scenario referenced only by unchecked tasks,
or by no task at all, is pending.

## Effect on the audit verdict

Pending scenarios are reported for visibility but never count toward the audit's error
total, so a change still in progress does not fail its scoped audit. Only error-classified
scenarios — claimed but unlinked — contribute to the failure verdict. A scoped audit with
no errors and no pending scenarios means every `test:code` scenario the change introduces
is implemented and linked: the change is ready to archive.

## Scope

The classification applies only when auditing a single change. A full-project audit does
not consult step completion: an unlinked `test:code` scenario in the main capabilities is
always an error, and a full audit never produces pending scenarios. The pending category
exists because an in-flight change has step state to consult; the main capability tree
does not.
