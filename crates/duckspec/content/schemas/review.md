# Review schema

A review is an **advisory critique** of a change — a fresh, adversarial read of
the change against its own contract and its diff. It records judgment a reader or
agent can act on, but it gates nothing: no command blocks on a review. Reviews
live as document-schema artifacts under `changes/<name>/reviews/NN-<slug>.md` and
form an append-only, chronological log.

## Structure

```markdown
# <Review Title>

<1-2 sentence summary: what was reviewed, against what, and the headline verdict>

## Scope

<what this review covers — the artifacts and/or diff examined, and from what
angle (design critique, post-implementation code review, spec-vs-code drift)>

## Findings

### <Finding title> — <severity>

<what the issue is, where it lives (`path:line` when it's code), why it matters,
and the recommended action>

## Verdict

<the overall judgment and what should happen next — advisory, not a gate>
```

## Severity

Tag each finding with one of:

- **blocker** — the change is wrong or unsafe to ship as-is; fix before archiving.
- **major** — a real problem worth fixing, but not strictly disqualifying.
- **minor** — a small improvement, cleanup, or nit.
- **question** — something unclear that needs an answer, not necessarily a fix.

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended, not
  enforced by `ds check`. A review validates against the document schema only.

## Quality

- **Adversarial, not ceremonial.** A review earns its place by finding things.
  Read the change against its proposal/spec contract and against the actual diff;
  look for drift, missed scenarios, unsafe assumptions, and untested edges. A
  review that only says "looks good" wasn't worth writing.
- **Code quality is the main job.** Correctness is the floor. Weigh the diff for
  simplicity (the smallest solution that works — no accidental complexity,
  speculative generality, or abstraction that doesn't pay for itself), code
  smells (duplication, over-long functions, leaky abstractions, misplaced logic),
  idiom (does it read like this codebase and language?), and design fidelity (does
  the code realize the design or quietly diverge?). When you flag complexity, name
  the simpler shape.
- **Each finding is actionable.** Name the artifact or `path:line`, state why it
  matters, and recommend a concrete action. A finding the reader can't act on is
  an observation, not a finding.
- **Severity is honest.** Reserve **blocker** for things that are genuinely wrong
  or unsafe. Inflating severity trains readers to ignore it.
- **The verdict commits.** Say plainly whether the change is ready, needs work,
  or raises open questions — then let the human decide. The review advises; it
  never gates.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to fences
that contain real code.

## Example

```markdown
# Post-implementation review: Google OAuth login

Reviewed the `auth/google` implementation against its spec and the change diff.
One blocker around token validation, otherwise solid.

## Scope

The `caps/auth/google` spec, the change's steps, and the diff under
`src/auth/google/`. Read as a post-implementation code review.

## Findings

### State parameter is not validated on callback — blocker

`src/auth/google/callback:42` reads the `state` query param but never compares
it to the value stashed at authorize time, leaving the flow open to CSRF. The
spec's "Callback rejects a forged state" scenario has a backlink but the test
asserts only on a present state, not a matching one. Validate equality and tighten
the test.

### Token refresh path is untested — major

`refresh_token` has no `@spec` coverage and no unit test. Add a scenario or at
least a direct test before archiving.

## Verdict

Not ready to archive: resolve the state-validation blocker and add refresh
coverage first. The rest of the flow matches the spec.
```
