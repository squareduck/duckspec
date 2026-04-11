# duckspec

duckspec is a spec-driven development framework. Capabilities are
described by paired spec and doc files, grouped into a capability
tree. Cross-cutting project knowledge lives in a separate codex.
Changes are proposed, reviewed, and applied through a structured
workflow. Code tests link back to the scenarios they verify, making
spec drift detectable.

## Directory layout

All duckspec artifacts live under a single `duckspec/` directory at
the root of the project. This directory is committed to version
control.

```
duckspec/
  project.md                       (optional)
  caps/
    <capability-path>/
      spec.md
      doc.md
  codex/
    <entry-path>.md
  changes/
    <change-name>/
      proposal.md                  (optional)
      design.md                    (optional)
      caps/                        (optional)
        <capability-path>/
          spec.md                  (new capability, full file)
          spec.delta.md            (modification of existing capability)
          doc.md
          doc.delta.md
      steps/                       (optional)
        NN-<slug>.md
  archive/
    YYYY-MM-DD-NN-<change-name>/
```

- **`project.md`** — optional project constitution: project-wide
  principles and constraints loaded into agent context by command
  templates. Edited directly, never carried through a change.
- **`caps/`** — the tree of capabilities. Each capability is a
  folder containing a `spec.md` and a `doc.md`. Subdirectories form
  nested capability paths and may also act as pure grouping
  namespaces. The name `caps/` is an abbreviation of "capabilities"
  used in paths and CLI flags; prose always says "capability".
- **`codex/`** — cross-cutting narrative knowledge: architecture
  overviews, domain glossaries, design philosophy, project-wide
  rationale. A flat tree of markdown files, edited directly.
- **`changes/`** — in-progress changes, one folder per change. Each
  change contains any subset of proposal, design, capability
  modifications, and implementation steps.
- **`archive/`** — frozen, applied changes in date-ordered folders.

`project.md` and `codex/` are never carried through a change. They
are edited directly in the working copy, and their history is
tracked only by version control. Changes do not contain a `codex/`
or `project.md`.

## Workflow

Users drive duckspec through agent command templates, invoked from
their AI harness (such as Claude Code or OpenCode). Each command
loads a template into the agent's context, and the agent guides the
user through that phase with assistance from the `ds` CLI.

Any phase can be skipped when the change doesn't need it. A
documentation-only change skips planning, speccing code behavior, and
stepping. A proposal-only change skips everything after planning. The
workflow adapts to the work.

### Flows

**Full feature change.** A new feature that introduces capabilities
and code.

```
/ds-explore   → orient, identify that a new change is needed
/ds-plan      → create proposal.md and design.md
/ds-spec      → create new capability specs and docs in changes/<name>/
/ds-step      → break the work into sequential steps
/ds-apply     → implement the first step with unfinished tasks
/ds-apply     → (next session) implement the next step
/ds-apply     → (repeat until all steps complete)
/ds-archive   → validate, apply to caps/, move to archive/
```

**Documentation-only change.** Updating the per-capability narrative
doc for an existing capability without changing behavior.

```
/ds-explore   → orient, identify a doc update is needed
/ds-spec      → create caps/<path>/doc.delta.md in the change
/ds-archive   → apply doc delta and archive
```

Planning and stepping are skipped because no new behavior is being
introduced and no code changes are required.

**Proposal-only change.** Capturing an idea for future work without
committing to implementation.

```
/ds-explore   → orient
/ds-plan      → write proposal.md describing the idea
/ds-archive   → archive the proposal for future reference
```

The change contains only a proposal. Archiving moves it to `archive/`
with no files applied to top-level.

**Spec refinement.** Clarifying or correcting existing capability
specs without code changes.

```
/ds-explore   → orient
/ds-spec      → create spec deltas for the affected capabilities
/ds-archive   → apply spec deltas and archive
```

No planning, no stepping, no code.

**Knowledge harvest.** Capturing cross-cutting learnings into the
codex.

```
/ds-explore   → gather context on what was learned in this session
/ds-codex     → draft new or updated codex entries and write directly
                to duckspec/codex/
```

Knowledge harvest is not tied to any change. Codex entries are
written directly to the top-level `codex/` tree; no delta, no
archive, no change wrapping.

**Resuming an in-progress change.** Continuing work on an existing
change in a new agent session.

```
/ds-explore   → detect the existing change, report its phase
/ds-apply     → the CLI identifies the first step with unfinished
                tasks; the agent implements it
```

The current step is always derivable from the filesystem: it is the
first step file (by numeric prefix) whose `## Tasks` list contains
any unchecked items.

### Commands

| Command       | Purpose                                                        |
|---------------|----------------------------------------------------------------|
| `/ds-explore` | Gather context for any duckspec activity; report current state |
| `/ds-plan`    | Author proposal.md and design.md                               |
| `/ds-spec`    | Author capability specs and docs (full files or deltas)        |
| `/ds-step`    | Break the change into sequential step files                   |
| `/ds-apply`   | Implement the first step with unfinished tasks                 |
| `/ds-archive` | Validate, apply to top-level, and archive the change           |
| `/ds-verify`  | Phase-aware verification with CLI assistance                   |
| `/ds-codex`   | Draft or update codex entries directly in `duckspec/codex/`    |

`/ds-explore` is the universal context-gathering entry point: it is
used before planning a change, before resuming one, and before any
knowledge-harvest work with `/ds-codex`. Every other command assumes
the agent has recent exploration context.

`/ds-codex` writes directly to the top-level `codex/` tree. It does
not interact with the change workflow, does not produce deltas, and
is not gated by `/ds-archive`. It is a side command, like
`/ds-verify`.

## Capabilities

A **capability** is a unit of observable behavior: a coherent piece
of what the system does for users or other systems. Each capability
is represented as a folder under `caps/` containing two required
files:

- `spec.md` — the formal behavior contract: requirements and
  scenarios.
- `doc.md` — the human-language narrative context for the
  capability.

Both files are required after archive. During an in-progress change,
either file may be absent: a change that only modifies the spec
contains only a `spec.delta.md`; a change that only adds narrative
context contains only a `doc.delta.md` or a full `doc.md`.

### Capability paths

A capability's **path** is the folder's path relative to `caps/`,
using forward slashes. Examples:

```
caps/auth/spec.md                  → capability "auth"
caps/auth/google/spec.md           → capability "auth/google"
caps/payments/stripe/refund/spec.md → capability "payments/stripe/refund"
```

Capability paths can nest arbitrarily. Path segment rules:

- No whitespace in any segment.
- Kebab-case is recommended for multi-word segments (`email-password`,
  not `emailPassword` or `email_password`).
- Segments should be short and descriptive.

Depth is not enforced by `ds check`. Paths deeper than three levels
are discouraged and may surface as a lint warning in a future
release, but they remain valid; agents can respect the warning as a
nudge toward flatter layouts.

### Grouping directories

A directory under `caps/` that does **not** contain a `spec.md` is a
**grouping directory**: a pure namespace node, not a capability
itself. Grouping directories carry no metadata and exist only to
hold descendant capabilities.

```
caps/
  payments/
    stripe/
      spec.md        ← capability "payments/stripe"
      doc.md
    paypal/
      spec.md        ← capability "payments/paypal"
      doc.md
```

Here `payments/` is a grouping directory; `stripe/` and `paypal/`
are capabilities. Grouping directories are created implicitly when a
nested capability is created, and disappear implicitly when their
last descendant capability is removed.

### Capability and descendants under the same name

A capability may coexist with descendant capabilities under the same
name. A folder may contain both a `spec.md` and subdirectories with
their own `spec.md`:

```
caps/
  auth/
    spec.md          ← capability "auth" (cross-cutting auth behavior)
    doc.md
    google/
      spec.md        ← capability "auth/google" (Google-specific path)
      doc.md
    email-password/
      spec.md        ← capability "auth/email-password"
      doc.md
```

In this layout, `caps/auth/spec.md` describes cross-cutting behavior
that applies to any authenticated session — session expiration,
logout, etc. — while `caps/auth/google/spec.md` and
`caps/auth/email-password/spec.md` describe the specifics of each
authentication path. All three are independent capabilities with
their own requirements and scenarios.

The parent capability `auth` and its children `auth/google`,
`auth/email-password` are not linked by any framework mechanism;
they share a path prefix, and that is all. Any cross-capability
references are written explicitly in prose or through `@spec`
backlinks.

### Capability pairing

Within a capability folder, `spec.md` and `doc.md` are paired by
their shared location in the folder. There is no explicit
cross-reference between the two files; the pairing is purely
structural. An archived capability folder that contains only one of
the two files is a validation error.

The `doc.md` file may be minimal — an H1 and a summary paragraph
with no further content is valid. The doc provides narrative context
through its existence and summary, even when it has no additional
sections. The `spec.md` carries the behavior contract.

The `spec.md` and `doc.md` files share the same H1 title. `ds check`
verifies that they match exactly.

### Summary and description

Every duckspec artifact that has an H1 (capability spec, capability
doc, proposal, design, step, codex entry, `project.md`) begins with
a **summary paragraph** directly after its H1, and may optionally
include a **description** of zero or more freeform markdown blocks
between the summary and the next heading.

The **summary** is:

- required,
- a single paragraph directly after the H1,
- non-empty,
- allowed to use inline markdown formatting (bold, italic, links,
  code spans) but not block elements (lists, quotes, code blocks),
- typically one or two plain sentences describing the artifact.

The summary is used by `ds index` to produce a project-wide overview
and by agents to quickly orient on what an artifact is about.

The **description** is:

- optional,
- zero or more markdown blocks (paragraphs, lists, quotes, code
  blocks) appearing between the summary paragraph and the next
  heading,
- structurally unconstrained beyond "must be parseable markdown" —
  parsers do not enforce any further shape on description content,
- intended as freeform elaboration: extra context, rationale,
  caveats, or pointers that would make the summary too long.

Description applies uniformly across every artifact type. In a
capability spec it appears between the summary and the first
`## Requirement:`. In a capability doc, codex entry, or
`project.md` it appears between the summary and the first H2 (or
fills the rest of the file if there are no further headings). In a
proposal, design, or step it appears between the summary and the
first required section heading.

Example (capability spec with a description):

```markdown
# Authentication

Allows users to sign in with email and password. Primary auth
mechanism for consumer accounts.

This capability is the foundation for all user-facing access
control. It covers the happy path (credentials accepted) and the
most common failure modes (credentials rejected, account locked).
Delegated auth via third-party providers is a separate capability
under `auth/oauth/`.

## Requirement: Email-password login
...
```

Example (capability spec without a description — description is
optional):

```markdown
# Health check

The system exposes a basic liveness probe so monitoring systems can
verify the service is running.

## Requirement: Liveness endpoint
...
```

### File dispatch by extension

Inside a capability folder (under `caps/` or under
`changes/<name>/caps/`), files are dispatched to the parser by
filename:

- `spec.md` — full capability spec. Parsed with the spec schema.
- `spec.delta.md` — spec delta. Parsed with the delta schema.
- `doc.md` — full capability doc. Parsed with the doc schema.
- `doc.delta.md` — doc delta. Parsed with the delta schema.

Other filenames inside a capability folder are ignored by duckspec
and reserved for user notes or future extensions.

A **full file** describes the entire spec or doc as-is. A **delta
file** describes a set of modifications to apply to an existing spec
or doc. The two schemas are related but distinct; the parser
selects the correct one by filename.

Full files can appear in `caps/` (the current state) and in
`changes/<name>/caps/` (introducing a new capability or replacing an
existing one). Delta files can appear **only** in
`changes/<name>/caps/`.

**Replace semantics for full files in a change.** A full `spec.md`
or `doc.md` in a change folder replaces the entire file at archive
time:

- If the capability does not yet exist at the top level, the full
  file introduces a new capability.
- If the capability already exists at the top level, the full file
  **replaces** its `spec.md` or `doc.md` entirely. This is the
  escape hatch for capabilities that need a wholesale rewrite
  rather than a delta.

**Exclusivity.** Within a single change, a capability file is
carried by **either** a full file or a delta file, not both. A
`changes/<name>/caps/auth/spec.md` and a
`changes/<name>/caps/auth/spec.delta.md` coexisting in the same
change is a validation error.

A delta file targeting a capability that does not exist at the top
level is also a validation error: deltas modify; they do not create.

The extension-based dispatch applies only inside capability folders
under `caps/` (or their mirrors in `changes/<name>/caps/`). Inside
`codex/`, filenames have no special meaning: see the Codex section.

### Spec schema

A capability spec describes the behavior contract for the
capability: what the system must do, expressed as a set of
requirements and scenarios.

Structure:

```markdown
# <Capability Title>

<1-2 sentence summary>

## Requirement: <requirement name>

<optional normative prose: SHALL/MUST/SHOULD statements>

> test: code    (optional, sets default for child scenarios)

### Scenario: <scenario name>

- **GIVEN** ...
- **WHEN** ...
- **THEN** ...
- **AND** ...

> test: code    (optional, overrides requirement default)
```

Rules:

- H1 title is required.
- A summary paragraph follows the H1.
- All H2 headers must start with the literal prefix `Requirement: `.
  Any other H2 is a validation error.
- Under each `## Requirement:` header, all H3 headers must start
  with the literal prefix `Scenario: `. Any other H3 is a
  validation error.
- A requirement must have either normative prose in its body, at
  least one scenario, or both. An empty requirement is a validation
  error.
- No headings H4 or deeper are allowed anywhere in a spec.
- Requirement names must not contain colons.
- Scenario names may contain colons.

**Example 1 — minimal spec with one requirement and one scenario:**

```markdown
# Session expiration

Sessions expire after a period of inactivity to reduce the blast
radius of stolen tokens.

## Requirement: Idle timeout

The system SHALL expire authenticated sessions after 30 minutes of
inactivity.

### Scenario: Idle user

- **GIVEN** an authenticated user
- **WHEN** the user makes no requests for 30 minutes
- **THEN** the next request returns 401
- **AND** the session token is invalidated server-side

> test: code
```

**Example 2 — requirement-level test marker inheritance:**

```markdown
# Email-password authentication

Users authenticate by submitting their email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **GIVEN** a user with a registered email and correct password
- **WHEN** the user submits the login form
- **THEN** the system issues a session token
- **AND** the user is redirected to their home page

### Scenario: Invalid password

- **GIVEN** a user with a registered email
- **WHEN** the user submits an incorrect password
- **THEN** the system rejects the login
- **AND** displays a generic "invalid credentials" error
- **AND** does not reveal whether the email is registered

### Scenario: Visual correctness of login button

- **GIVEN** the user is on the login page
- **WHEN** the page finishes loading
- **THEN** the sign-in button is visible and correctly styled

> manual: visual check during release review
```

Both the `Valid credentials` and `Invalid password` scenarios
inherit `test: code` from the requirement. The `Visual correctness
of login button` scenario overrides the inherited marker with its
own `manual:` marker.

**Example 3 — multiple requirements:**

```markdown
# Authentication

Allows users to sign in with email and password, manages their
sessions, and provides logout.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

### Scenario: Invalid password

- **WHEN** the user submits an incorrect password
- **THEN** the system rejects the login with a generic error

## Requirement: Session expiration

The system SHALL expire idle sessions after 30 minutes.

> test: code

### Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** 30 minutes pass with no requests
- **THEN** the next request returns 401

## Requirement: Logout

The system SHALL allow users to explicitly invalidate their session.

> test: code

### Scenario: Explicit logout

- **WHEN** the user submits a logout request
- **THEN** the session token is invalidated
- **AND** future requests with that token are rejected
```

### Scenario grammar

A scenario is an H3 header followed by **exactly one** unordered
list of GWT (Given-When-Then) bullets, optionally followed by a
test marker blockquote. Nothing else is allowed in a scenario's
body: no description paragraphs, no additional lists, no code
blocks. Any other content is a validation error — if you want to
elaborate on a scenario, put the prose in the parent requirement's
description or in the capability doc.

Recognized clause keywords, in uppercase and bold:

- `**GIVEN**` — initial context or state (optional)
- `**WHEN**` — the trigger or action
- `**THEN**` — the expected outcome
- `**AND**` — continuation of any prior clause

Every scenario must contain at least one `**WHEN**` clause and at
least one `**THEN**` clause. Scenarios without both are a validation
error.

Clauses are written as unordered list items. Each bullet begins
with the bold keyword, followed by prose describing the clause.

**Example — scenario with all four clause types:**

```markdown
### Scenario: Failed login lockout

- **GIVEN** a user account with three prior failed login attempts
- **WHEN** the user submits a fourth incorrect password
- **AND** the submission occurs within one hour of the first failure
- **THEN** the account is locked for 15 minutes
- **AND** subsequent login attempts are rejected without checking
  the password
```

**Example — minimal scenario with only WHEN and THEN:**

```markdown
### Scenario: Health check

- **WHEN** a client requests GET /health
- **THEN** the system returns 200 OK
```

### Test marker

Every scenario must resolve to a test marker that declares how the
scenario is verified. The marker appears in a blockquote directly
after the scenario's GWT bullets, or is inherited from the
requirement that contains it.

Three marker prefixes are recognized:

- `> test: code` — the scenario is verified by automated code tests.
  After `ds sync`, sub-blockquote bullets list the paths of the
  linked tests.
- `> manual: <reason>` — the scenario is verified manually. The
  reason is required and describes how the manual verification
  happens.
- `> skip: <reason>` — the scenario is intentionally not verified.
  The reason is required and explains why.

A scenario's marker is the one written directly in its blockquote.
If the scenario has no blockquote, it inherits the marker from its
parent requirement. If neither the scenario nor the requirement has
a marker, validation fails.

**Example — `test: code` marker:**

```markdown
### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

> test: code
```

**Example — `manual:` marker with reason:**

```markdown
### Scenario: Accessibility of login form

- **GIVEN** a user with a screen reader
- **WHEN** the user navigates to the login page
- **THEN** all form fields are reachable and labeled

> manual: accessibility audit performed per release
```

**Example — `skip:` marker with reason:**

```markdown
### Scenario: Legacy password reset flow

- **WHEN** a user clicks "forgot password" on the legacy page
- **THEN** they are redirected to the new flow

> skip: covered by redirect integration test; scenario retained for
  documentation
```

**Example — requirement-level inheritance:**

```markdown
## Requirement: OAuth token refresh

The system SHALL refresh expired OAuth tokens transparently.

> test: code

### Scenario: Token near expiration

- **GIVEN** an access token within 5 minutes of expiration
- **WHEN** the user makes an authenticated request
- **THEN** the system refreshes the token before processing

### Scenario: Refresh token revoked

- **GIVEN** a revoked refresh token
- **WHEN** the user makes an authenticated request
- **THEN** the system returns 401 and requires re-authentication
```

Both scenarios inherit `test: code` from the requirement.

### Sync augmentation

The `ds sync` command scans source files for `@spec` backlinks (see
**Code backlinks** below), matches them to scenarios, and augments
`> test: code` blockquotes in spec files with the file paths of
linked tests.

Before sync:

```markdown
### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

> test: code
```

After sync:

```markdown
### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

> test: code
> - crates/auth/tests/login.rs:42
> - crates/auth/tests/integration.rs:117
```

Multiple backlinks to the same scenario from different source
locations are all listed as sub-blockquote bullets. At least one
linked path must exist for every `test: code` scenario after sync;
scenarios with zero links are a validation error at audit time.

Sync is mutating: it rewrites spec files to include the resolved
paths. `ds sync --dry` prints the changes that would be made without
writing them.

Only `test: code` markers are augmented by sync. `manual:` and
`skip:` markers are not augmented; they are declarative only.

### Spec delta schema

A spec delta file at `changes/<name>/caps/<capability-path>/spec.delta.md`
describes a set of modifications to apply to the existing spec at
`caps/<capability-path>/spec.md` when the change is archived.

A delta file has the same overall shape as a full spec, except
every header up to and including H3 carries an explicit marker. The
markers declare what operation to perform on the matching header in
the source spec.

Structure:

```markdown
# <marker> <Capability Title>

<optional new summary paragraph>

## <marker> Requirement: <requirement name>

<optional body>

### <marker> Scenario: <scenario name>

<optional body>
```

Every H1, H2, and H3 in a delta file must carry a marker. An
unmarked header is a validation error. See **Delta markers** below
for the marker set and semantics.

If the delta file contains a new summary paragraph after the H1, it
replaces the source summary when the delta is applied. If no summary
is present, the source summary is preserved. A summary cannot be
removed through a delta.

**Example — purely additive delta:**

```markdown
# @ Authentication

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### + Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
- **AND** a QR code is displayed for the authenticator app

### + Scenario: 2FA verification

- **GIVEN** a user with 2FA enabled
- **WHEN** the user logs in with valid credentials
- **THEN** the system requests a TOTP code
- **AND** grants access only if the code is valid
```

Adds a new `Two-factor authentication` requirement with two
scenarios. The capability's existing requirements are untouched.

**Example — delta with multiple operations:**

```markdown
# @ Authentication

## = Requirement: Email-password login

Email-password authentication

## - Requirement: Remember me

## @ Requirement: Session expiration

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** the current session remains valid
- **AND** all other sessions for that user are invalidated

> test: code
```

Renames `Email-password login` to `Email-password authentication`,
removes the entire `Remember me` requirement, and adds a new
scenario to the existing `Session expiration` requirement. The
entries appear in canonical order: `=` first, then `-`, then `@`.

### Doc schema

A capability doc provides human-language narrative context for a
capability: overview, background, design decisions, user journeys,
open questions, rationale, or whatever fits. Unlike specs, docs are
freeform after the required preamble.

Structure:

```markdown
# <Capability Title>

<1-2 sentence summary>

<freeform markdown content>
```

Rules:

- H1 title is required and must match the paired spec's H1 exactly.
- A summary paragraph follows the H1.
- The body may contain any markdown: any H2/H3/H4 headers, any
  prose, lists, code blocks, quotes, images, links.
- No validation beyond the H1 and summary.

**Example 1 — minimal doc with only H1 and summary:**

```markdown
# Authentication

Allows users to sign in with email and password. Primary auth
mechanism for consumer accounts.
```

This is valid. The doc provides context through its existence and
summary, even though it has no additional sections. The paired spec
carries the behavior contract.

**Example 2 — doc with several freeform sections:**

```markdown
# Authentication

Allows users to sign in with email and password. Primary auth
mechanism for consumer accounts.

## Background

Email-password was chosen over username-password to align with how
users think about identity and to simplify onboarding. Social login
providers are deferred to a later phase.

## User journey

A new user lands on the marketing site, clicks "Sign up," enters
their email and chooses a password, receives a verification email,
and completes registration by clicking the verification link.
Returning users click "Sign in" and enter the same credentials.

## Design decisions

- **Session duration**: 30 minutes of inactivity. Short enough to
  limit the blast radius of stolen tokens, long enough to avoid
  disrupting active users.
- **Error messages on invalid login**: generic "invalid credentials"
  regardless of which field was wrong, to prevent user enumeration.
- **Password storage**: argon2id with per-user salt.

## Open questions

- Should we offer "remember me" for trusted devices?
- What's the right lockout policy for repeated failed attempts?
```

### Doc delta schema

A doc delta file at `changes/<name>/caps/<capability-path>/doc.delta.md`
describes a set of modifications to apply to the existing doc at
`caps/<capability-path>/doc.md`. Like spec deltas, every H1/H2/H3
must carry a marker. Unlike spec deltas, the content under each
header is freeform.

**Example:**

```markdown
# @ Authentication

## + Security rationale

Email-password was chosen over social-only login because consumer
users often distrust third-party identity providers and because it
allows us to operate without OAuth provider dependencies.

## ~ Session duration

Sessions expire after 30 minutes of inactivity. This was chosen to
balance security (limiting the blast radius of a stolen token)
against user experience (avoiding frequent re-authentication for
active users). A future change may introduce "remember me" for
trusted devices.
```

Adds a new `Security rationale` section and replaces the body of
the existing `Session duration` section. All other sections in the
source doc are preserved.

### Delta markers

Delta files use a small set of markers on their headers to declare
what operation to perform on the matching header in the source
file. The same marker set applies to spec deltas and doc deltas.

The markers are:

| Marker | Name    | Operation                                          |
|--------|---------|----------------------------------------------------|
| `+`    | add     | Insert a new header and its body into the source  |
| `-`    | remove  | Delete the matching header and its entire subtree |
| `~`    | replace | Replace the matching header's body and children   |
| `=`    | rename  | Rename the matching header, preserving children   |
| `@`    | anchor  | Optionally replace body, preserve and modify children |

**Invariant**: each header name at a given level appears at most
once in the delta file. For renames, the old name (in the `=`
entry) and the new name (in any subsequent `@`, `~`, `+`, or `-`
entry) are two distinct strings, and both may appear. No two
entries at the same level use the same name.

**Semantics**:

**`+` add.** The marked header does not exist in the source. The
delta inserts it as a new header with the body written in the
delta. If a header with the same text already exists in the source
at the same level, it is a validation error: you cannot add what
already exists. Use `~` or children markers instead.

```markdown
### + Scenario: New edge case

- **WHEN** an unusual condition occurs
- **THEN** the system handles it gracefully

> test: code
```

**`-` remove.** The marked header exists in the source and is
removed, along with its entire subtree (all nested headers and
content). The body written in the delta under a `-` header is
ignored; only the header itself is meaningful.

```markdown
## - Requirement: Deprecated login flow
```

The body of a `-` marker must be empty. A non-empty body on a
remove entry is a validation error.

**`~` replace.** The marked header exists in the source. Its body
and all its children are replaced by what is written in the delta.
The header text itself is unchanged.

```markdown
### ~ Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** the user makes no requests for 60 minutes
- **THEN** the session expires

> test: code
```

The source's `Idle timeout` scenario is replaced with the new body
(note the changed duration). The header text `Idle timeout` is
preserved.

**`=` rename.** The marked header exists in the source. The entry
renames it: the header text is changed from the old name (written
on the marker line) to the new name (written as the first non-blank
line after the header). The rename is the entry's entire purpose —
it performs no other modification.

```markdown
## = Requirement: Email-password login

Email-password authentication
```

The `=` entry consists of exactly two meaningful lines: the marker
line with the old name, and the new-name line. Blank lines around
the new-name line are permitted for readability. The entry contains
no body paragraphs, no child headers, no blockquotes, and no other
content. Any such content is a validation error.

To modify the renamed header's body or children, write a separate
entry that uses the **new name** and a content-modifying marker:

```markdown
## = Requirement: Email-password login

Email-password authentication

## @ Requirement: Email-password authentication

### ~ Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token
- **AND** records the successful authentication

> test: code
```

The first entry renames the requirement. The second entry, using
the new name, anchors into the renamed requirement and replaces one
of its scenarios. Other scenarios under the requirement, if any,
are preserved unchanged.

Rename chains are forbidden. A source header may be renamed at most
once per delta. Renaming a header that was just created by another
rename in the same delta is a validation error.

After all renames are applied, every header name at each level must
be unique. A rename whose new name collides with an existing header
at the same level is a validation error.

**`@` anchor.** The marked header exists in the source. The
anchor marker serves two purposes depending on whether the delta
entry carries body content:

- **Without body:** the source header's body is preserved
  exactly as-is. The anchor is purely navigational — it gives the
  delta a place to attach child-level markers without disturbing
  the parent's content.
- **With body:** the source header's body (summary/description
  for H1, normative prose and/or test marker for H2) is replaced
  with the delta entry's body. The header text and children are
  preserved; child markers are applied recursively as usual.

This makes `@` the marker for "update my body, keep my children,"
filling the gap between `~` (replace everything) and a pure
navigation anchor.

The `@` marker is not valid on H3 (scenario) headings. Scenarios
have no children to anchor into — use `~` to replace a scenario's
content or `-` to remove it.

```markdown
## @ Requirement: Session expiration

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated

> test: code
```

The `Session expiration` requirement's body is preserved (no body
on the `@` entry); the delta only adds a new scenario under it.

An anchor with body replaces only the source header's prose while
preserving its children:

```markdown
## @ Requirement: Session expiration

The system SHALL expire idle sessions after 15 minutes of inactivity.

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated

> test: code
```

The `Session expiration` requirement's normative prose is replaced
with the new text. Existing scenarios under it are preserved, and
the new `Force logout on password change` scenario is added.

An anchor may also carry a test marker blockquote to replace the
source requirement's marker:

```markdown
## @ Requirement: Session expiration

The system SHALL expire idle sessions after 15 minutes of inactivity.

> test: code

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated
```

The `Session expiration` requirement's normative prose and test
marker are both replaced. Its existing scenarios are preserved and
the new scenario is added.

**Whole-capability operations on H1.** The H1 header of a delta
file follows the same marker rules:

- `# @ <Title>` — modify children of the capability; if a summary
  or description follows the H1, it replaces the source's
  summary/description (the common case)
- `# ~ <Title>` — replace the entire capability with the delta's
  content, including summary and all requirements
- `# - <Title>` — delete the entire capability
- `# = <Old Title>` with a new title on the next line — rename the
  capability's H1
- `# + <Title>` — this is invalid; use a full file (`spec.md` or
  `doc.md`) instead of a delta to create a new capability file

**Canonical order.** Within each header level, delta entries appear
in a fixed canonical order:

1. `=` rename entries, in the source order of their old names
2. `-` remove entries, in the source order of their targets
3. `~` replace entries, in the source order of their targets
4. `@` anchor entries, in the source order of their targets
5. `+` add entries, in the order the author wants them appended

The same rule applies recursively: inside each `@` anchor, the
children follow the same five-group order at their own level.

Canonical order is both the apply order and the required file
order. Entries out of canonical order are a validation error: `ds
check` reports them, and `ds check --format` rewrites the file to
canonical order.

Example of a correctly ordered delta:

```markdown
# @ Authentication

## = Requirement: Login

Email-password login

## - Requirement: Remember me

## ~ Requirement: Session expiration

The system SHALL expire idle sessions after 15 minutes of inactivity.

> test: code

### Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** the user makes no requests for 15 minutes
- **THEN** the next request returns 401

## @ Requirement: Email-password login

### + Scenario: Account lockout

- **GIVEN** three consecutive failed login attempts
- **WHEN** the user submits a fourth incorrect password
- **THEN** the account is locked for 15 minutes

> test: code

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
- **AND** a QR code is displayed for the authenticator app
```

The delta's H2 entries appear in canonical order: `=` first
(renaming `Login`), then `-` (removing `Remember me`), then `~`
(replacing `Session expiration`), then `@` (anchoring into the
already-renamed `Email-password login`), then `+` (adding a new
`Two-factor authentication` requirement).

### Applying deltas

Applying a delta produces a new full-file version of the source.
The apply algorithm is type-agnostic: it operates on the tree of
marked headers without knowledge of whether the content is a spec
or a doc.

The apply process, in order:

1. **Parse the source file** into a tree of headers and body
   content. Each node records its header text (without any
   marker), its level, its direct body, and its children.
2. **Parse the delta file** into a tree of marked headers. Each
   node records its marker, its header text (without the marker),
   its level, its body, and its children. Unmarked headers in a
   delta are a validation error.
3. **Walk the delta tree.** For each marked node, locate the
   matching node in the source tree by its header text at the same
   level and ancestor path. The match must be exact
   (case-sensitive, whitespace-trimmed).
4. **Apply operations in canonical order.** Within each level,
   operations are applied in the following sequence:
   - `=` renames first. Each rename changes the matching source
     node's header text from the old name to the new name; the
     node's body and children are preserved. After all renames are
     applied, subsequent entries can reference nodes by their new
     names.
   - `-` removals next. Each removal deletes the matching source
     node and its subtree.
   - `~` replacements next. Each replacement replaces the matching
     source node's body and children with the delta entry's body
     and children; the header text is preserved.
   - `@` anchors next. If the anchor entry carries body content,
     the matching source node's body is replaced with the delta
     entry's body; otherwise the body is preserved. Then the
     algorithm descends into the matching source node's children
     and recursively applies the same ordered sequence to those
     children.
   - `+` additions last. Each addition creates a new child of the
     current level, appended after the last existing sibling at
     that level at the time of insertion.
5. **Validate the resulting tree** against the target type's schema
   (spec or doc). A delta that produces an invalid result — for
   example, a requirement with no prose and no scenarios after a
   `-` removed its only scenario — is a validation error.
6. **Serialize the resulting tree** to produce the new full-file
   version. Serialization uses a canonical order and formatting,
   regardless of how the delta was written. The serialization
   order matches how full files are read: H1, summary, then H2
   headers in their in-memory order, each with their children in
   order.

**Canonical ordering.** The delta's entries are already in
canonical order when `ds check` accepts the file. The source tree's
order is preserved for unchanged nodes. New nodes added by `+` are
appended after the last existing sibling at their level at the time
they are applied. Because apply is deterministic, applying the same
delta to the same source always produces the same result, and
serializing the result always produces the same canonical output.

**Walkthrough — simple addition:**

Source (`caps/auth/spec.md`):

```markdown
# Authentication

Allows users to sign in with email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token
```

Delta (`changes/add-lockout/caps/auth/spec.delta.md`):

```markdown
# @ Authentication

## @ Requirement: Email-password login

### + Scenario: Account lockout

- **GIVEN** three consecutive failed login attempts
- **WHEN** the user submits a fourth incorrect password
- **THEN** the account is locked for 15 minutes
```

Result after apply:

```markdown
# Authentication

Allows users to sign in with email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

### Scenario: Account lockout

- **GIVEN** three consecutive failed login attempts
- **WHEN** the user submits a fourth incorrect password
- **THEN** the account is locked for 15 minutes
```

**Walkthrough — rename with nested modification:**

Source (`caps/auth/spec.md`):

```markdown
# Authentication

Allows users to sign in.

## Requirement: Login

The system SHALL allow user authentication.

### Scenario: Valid credentials

- **WHEN** correct credentials
- **THEN** session issued

### Scenario: Invalid credentials

- **WHEN** incorrect credentials
- **THEN** generic error
```

Delta (`changes/rename-login/caps/auth/spec.delta.md`):

```markdown
# @ Authentication

## = Requirement: Login

Email-password login

## @ Requirement: Email-password login

### ~ Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token
- **AND** records the successful authentication
```

Result after apply:

```markdown
# Authentication

Allows users to sign in.

## Requirement: Email-password login

The system SHALL allow user authentication.

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token
- **AND** records the successful authentication

### Scenario: Invalid credentials

- **WHEN** incorrect credentials
- **THEN** generic error
```

The delta has two entries at the H2 level: a `=` rename of `Login`
to `Email-password login`, and an `@` anchor into the renamed
requirement to replace one of its scenarios. The rename applies
first, then the anchor descends into the renamed node and processes
the child scenario replacement. The requirement was renamed and one
scenario was replaced; the other scenario was preserved unchanged.

## Codex

The codex is a tree of cross-cutting narrative knowledge that
applies to the whole project or to multiple capabilities at once.
Codex entries describe things that do not belong to a single
capability: architectural overviews, domain glossaries, design
philosophy, project-wide rationale, engineering conventions, and
anything else worth canonicalizing as reference material.

The codex complements per-capability `doc.md` files: a capability
doc describes *that capability*, while a codex entry describes
something that spans capabilities or stands outside them entirely.

### Codex layout

Codex entries are loose markdown files under `duckspec/codex/`.
Each file is one entry. Subdirectories are pure grouping namespaces
and may be used freely to organize entries:

```
codex/
  architecture.md
  glossary.md
  philosophy.md
  architecture/
    data-flow.md
    error-handling.md
  domain/
    billing.md
    pricing-rules.md
```

In this example the codex contains seven entries: `architecture.md`,
`glossary.md`, `philosophy.md`, `architecture/data-flow.md`,
`architecture/error-handling.md`, `domain/billing.md`, and
`domain/pricing-rules.md`. The `architecture/` and `domain/`
directories are grouping namespaces.

A codex file and a grouping directory may share a basename. For
example, `codex/architecture.md` (a top-level entry describing the
overall architecture) can coexist with `codex/architecture/data-flow.md`
(a nested entry describing a specific aspect). They are a file and
a directory and the filesystem handles them as independent.

Codex entry path segments follow the same conventions as capability
path segments: no whitespace, kebab-case recommended.

### Codex entry schema

A codex entry uses the same schema as a capability doc:

```markdown
# <Entry Title>

<1-2 sentence summary>

<freeform markdown content>
```

Rules:

- H1 title is required.
- A summary paragraph follows the H1.
- The body may contain any markdown: any headers, any prose, lists,
  code blocks, quotes, images, links.
- No validation beyond the H1 and summary.

**Example — minimal codex entry:**

```markdown
# Domain glossary

Definitions of domain terms used across the product.
```

**Example — codex entry with freeform content:**

```markdown
# Data flow architecture

How data moves between the web client, API gateway, workers, and
persistent stores.

## Overview

The system uses a request-response web layer for synchronous
user-facing operations and a message-queue-backed worker layer for
asynchronous work. The boundary between the two is explicit: any
operation that takes longer than 500ms at p50 is pushed to the
worker layer.

## Synchronous path

1. Client request arrives at the API gateway.
2. Gateway validates auth and rate limits.
3. Request is forwarded to the appropriate service.
4. Service responds directly.

## Asynchronous path

1. Client request arrives at the API gateway.
2. Gateway validates auth and rate limits.
3. Service enqueues a job on the message queue.
4. Service returns a job ID to the client.
5. Worker picks up the job and processes it.
6. Client polls for the job's result (or subscribes via SSE).
```

### Codex rules

- Codex entries are edited **directly** in `duckspec/codex/`. They
  are not carried through a change; there is no
  `changes/<name>/codex/` subtree.
- There is no codex delta format. Updating a codex entry is a
  direct edit of the file.
- The codex is not touched by `ds archive`. It has no archive
  lifecycle.
- `ds check` validates each codex entry against the codex entry
  schema (H1, summary, parseable markdown). It does not enforce any
  additional structure inside the body.
- `ds index --codex` renders a tree of codex entries with their
  summaries inline.

### When to write a codex entry vs. a capability doc

| You are writing about | Where it goes |
|---|---|
| Behavior of one capability | `caps/<path>/doc.md` |
| Cross-capability rationale or overview | `codex/<path>.md` |
| Project-wide principles, values, constraints | `project.md` |
| Domain glossary or vocabulary | `codex/glossary.md` or similar |
| Architecture that spans capabilities | `codex/architecture.md` or nested |
| Decision that affects multiple capabilities | `codex/decisions/<name>.md` |

If in doubt, lean toward a capability doc when the subject is
naturally owned by one capability, and a codex entry when it is
not.

## Project constitution

`duckspec/project.md` is an optional file that captures project-wide
principles, constraints, conventions, and any other knowledge that
agents should always consider when working on the project. It plays
the role of a "constitution" for duckspec work: the things that
are true regardless of which capability or change you are looking
at.

The file is optional. Projects that don't need one can omit it
entirely. Projects that have one load it into agent context early in
every `/ds-*` command, so that project-wide constraints are always
present during agent reasoning.

### project.md schema

`project.md` uses the same schema as a codex entry:

```markdown
# <Project Name>

<1-2 sentence summary>

<freeform markdown content>
```

Rules:

- H1 title is required.
- A summary paragraph follows the H1.
- The body may contain any markdown.
- There is at most one `project.md` at `duckspec/project.md`. Any
  file named `project.md` elsewhere in `duckspec/` is ignored by
  duckspec's parser.

**Example:**

```markdown
# duckpond

duckpond is a spec-driven development framework written in Rust
2024, with a strong preference for small, composable crates and
explicit error handling.

## Engineering principles

- **Filesystem is the source of truth.** No metadata in frontmatter
  or sidecars; everything derivable from files.
- **Library first.** Features start as `duckpond` library APIs and
  are surfaced through `duckspec` or `duckboard` CLIs only after
  the library is stable.
- **Explicit error types.** Library code uses `thiserror` typed
  enums; binaries wrap with `anyhow`.

## Testing discipline

- Unit tests inline under `#[cfg(test)] mod tests`.
- Integration tests in `tests/` using snapshot testing via `insta`
  for structured values.
- Fixture-driven parser tests with paired positive/rule-coverage
  files.

## Out of scope

- GUI for the duckpond library itself (duckboard is a separate
  consumer).
- Cloud-hosted variants.
```

### project.md rules

- The file is **edited directly** in `duckspec/project.md`. It is
  never carried through a change; there is no
  `changes/<name>/project.md`.
- Its evolution is tracked only by version control history, not by
  the duckspec change workflow.
- `ds check` validates the file against the schema above. It does
  not interpret the body.
- `project.md` is not archived and has no archive lifecycle.

## Changes

A change is a proposed set of modifications to the project: new or
updated capabilities, new code, or capability doc updates. Each
change lives in its own folder under `changes/`, and may contain
any combination of proposal, design, capability modifications, and
implementation steps. Changes that don't need a particular artifact
simply don't include it.

Changes never carry codex entries or `project.md`. Those artifacts
are edited directly outside the change workflow.

### Change structure

Every change is a folder at `changes/<change-name>/`, where
`<change-name>` is a human-readable slug chosen by the user or the
agent. The folder may contain:

```
changes/<change-name>/
  proposal.md                    (optional)
  design.md                      (optional)
  caps/                          (optional)
    <capability-path>/
      spec.md                    (new or full-replace capability spec)
      spec.delta.md              (spec modification)
      doc.md                     (new or full-replace capability doc)
      doc.delta.md               (doc modification)
  steps/                         (optional)
    NN-<slug>.md                 (implementation steps)
```

All subfolders and files are optional. A change with only a
`proposal.md` is valid. A change with only `caps/*/doc.delta.md`
files is valid. A change with specs and steps but no proposal is
valid.

Multiple changes can exist concurrently. Each lives in its own
folder and is archived independently.

### Proposal

A proposal describes why a change is being made and what it will do
at a high level. It is the pitch for the work.

Structure:

```markdown
# <Change Title>

<1-2 sentence summary>

<freeform content>
```

Rules:

- H1 title is required.
- A summary paragraph follows the H1.
- The body is freeform markdown.

**Example:**

```markdown
# Add Google OAuth login

Introduce Google as a third-party login option to reduce signup
friction for new users.

## Why

Consumer users increasingly expect social login as an option.
Analytics show roughly 40% of signup drop-offs happen at the
password creation step. Offering Google OAuth removes that friction
for the largest segment of drop-offs.

## What changes

- A new capability `auth/google` with its spec and doc
- UI changes: a "Sign in with Google" button on the login and
  signup screens
- Backend changes: OAuth 2.0 flow, token exchange, and user linking
  logic

## Out of scope

- Apple Sign In (deferred)
- Account linking for existing email-password users (deferred)
- Administrative controls for disabling social login per tenant
```

### Design

A design document describes the technical approach for a change.
Like the proposal, it has minimal structure: H1, summary, freeform
body.

Structure:

```markdown
# <Change Title> — Design

<1-2 sentence summary>

<freeform technical content>
```

**Example:**

```markdown
# Add Google OAuth login — Design

Implements Google OAuth 2.0 as a new authentication path alongside
the existing email-password flow, using our existing session
management.

## Architecture

The OAuth flow is implemented as a new endpoint at `/auth/google`
that redirects to Google's authorization URL. Google redirects back
to `/auth/google/callback` with an authorization code. The callback
handler exchanges the code for tokens, fetches the user profile,
and either creates a new user or logs in an existing one.

## Data model

A new `oauth_identities` table links Google account IDs to internal
user IDs. A single internal user may have multiple OAuth identities
(one per provider) plus an optional email-password credential.

## Token storage

Google access tokens are not persisted. Refresh tokens are stored
encrypted with the user's row. The session itself uses our existing
opaque session token mechanism, not the Google tokens directly.

## Error handling

- **User denies authorization**: redirect to `/login` with a flash
  message.
- **Google API failure**: redirect to `/login` with a generic error
  and log the upstream error server-side.
- **Email conflict with existing user**: create the OAuth identity
  and link it to the existing user, without re-prompting.
```

### Step

A step is a self-contained unit of implementation work, sized to
fit comfortably in a single AI agent session. A change's work is
broken into one or more steps, and each step is processed in its
own `/ds-apply` invocation.

Step files live at `changes/<change-name>/steps/NN-<slug>.md`:

- `NN` is a two-digit zero-padded number that determines step order.
- `<slug>` is a kebab-case slug matching the step's H1 title (in
  slugified form).

Structure:

```markdown
# <Human-readable step name>

<1-2 sentence summary>

## Prerequisites          (optional)

- [ ] @step <other-step-slug>
- [ ] Freeform prerequisite text

## Context                (optional)

<freeform prose>

## Tasks                  (required, at least one task)
- [ ] 1. <Task description>
  - [ ] 1.1 <Subtask>
  - [ ] 1.2 <Subtask>
- [ ] 2. <Another task>

## Outcomes               (optional)

<freeform prose, populated during or after implementation>
```

Rules:

- H1 is the human-readable step name. Its slugified form must
  equal the slug in the filename.
- A summary paragraph follows the H1.
- `## Prerequisites`, `## Context`, and `## Outcomes` are optional.
- `## Tasks` is required and must contain at least one task.
- Tasks are unordered list items with checkboxes and numeric
  prefixes (`1.`, `2.`, `3.`, ...).
- Tasks may have subtasks, nested one level deep. Deeper nesting
  (sub-subtasks) is a validation error.
- A step is considered complete when all task and subtask
  checkboxes in `## Tasks` are checked.
- The current step for `/ds-apply` is the step file with the
  lowest `NN` prefix that has any unchecked tasks.

**Task content rules.** A task may contain either:

- Freeform text describing the work to do, or
- A single `@spec <capability-path> <Requirement>: <Scenario>`
  backlink, and nothing else.

A task whose entire content is a backlink is a scenario
implementation task: it indicates that the task's goal is to
implement and test the referenced scenario. Exactly one scenario
per such task. Every scenario marked `test: code` in the change's
capability specs must have a corresponding scenario implementation
task in some step. This is a validation rule enforced by `ds
audit`.

**Prerequisites content rules.** Prerequisites are informational
only; the CLI does not enforce them. A prerequisite item may be:

- `@step <other-step-slug>` — a reference to another step that
  should be completed before this one. Used by agents to understand
  dependencies when reading the step.
- Freeform text — a human-readable description of a prerequisite
  condition.

Both kinds may be mixed in the same prerequisites list. The agent
loading the step template reads the prerequisites and incorporates
them into its reasoning.

**Example 1 — minimal step with prose tasks:**

```markdown
# Add login form UI

Build the React login form component with email and password
fields and a submit handler.

## Tasks

- [ ] 1. Create `LoginForm` component in `src/components/auth/`
  - [ ] 1.1 Email input with validation
  - [ ] 1.2 Password input with visibility toggle
  - [ ] 1.3 Submit button with loading state
- [ ] 2. Wire up the form to the existing `/auth/login` API
- [ ] 3. Handle error responses and display messages
```

**Example 2 — step with scenario implementation tasks:**

```markdown
# Implement session expiration

Add server-side session timeout logic and cover the scenarios with
tests.

## Context

The session middleware currently does not track last-access time.
This step adds that tracking and the expiration check.

## Tasks

- [ ] 1. Add `last_accessed_at` column to the `sessions` table
- [ ] 2. Update session middleware to refresh `last_accessed_at` on
      each request
- [ ] 3. @spec auth Session expiration: Idle timeout
- [ ] 4. @spec auth Session expiration: Force logout on password change
```

Tasks 3 and 4 are scenario implementation tasks: each will result
in a test that backlinks to the referenced scenario. The test's
backlink comment is what `ds sync` later scans to populate the
scenario's `> test: code` blockquote.

**Example 3 — step with prerequisites and nested capability path:**

```markdown
# Integrate Google OAuth callback

Wire the OAuth callback handler into the authentication middleware
and session management.

## Prerequisites

- [ ] @step add-oauth-endpoints
- [ ] Google OAuth credentials are provisioned in the staging
      environment
- [ ] @step add-oauth-identities-table

## Tasks

- [ ] 1. Implement `/auth/google/callback` handler
- [ ] 2. Exchange authorization code for tokens
- [ ] 3. Fetch Google user profile
- [ ] 4. Look up or create the internal user
- [ ] 5. @spec auth/google OAuth callback: Valid callback
- [ ] 6. @spec auth/google OAuth callback: User denies authorization
- [ ] 7. @spec auth/google OAuth callback: Existing user linking
```

## Code backlinks

Code backlinks connect automated tests in the project's source to
the scenarios they verify. A backlink is a comment in a source file
whose first word (after the comment syntax) is `@spec`, followed by
a capability path, requirement name, and scenario name.

Format:

```
@spec <capability-path> <Requirement Name>: <Scenario Name>
```

The comment must be a valid comment in the source file's language.
The `@spec` token must be the first word of the comment's content —
leading whitespace after the comment marker is tolerated, but no
other content may precede `@spec`.

Parsing rules:

- The token immediately after `@spec` is the capability path.
  Capability paths contain no whitespace and use forward slashes
  for nested paths (e.g. `auth/google`).
- Everything after the capability path, up to the first colon, is
  the requirement name. Requirement names contain no colons.
- Everything after the first colon is the scenario name. Scenario
  names may contain additional colons.
- Matching is case-sensitive and exact, after collapsing runs of
  whitespace to single spaces.

Each backlink is recorded as a pair of `(file path, line number)`.
Multiple backlinks pointing to the same scenario from different
source files are all retained and shown in the spec after `ds
sync`.

**Example — Rust doc comment:**

```rust
/// @spec auth Email-password login: Valid credentials
#[test]
fn test_login_happy_path() {
    let response = login("user@example.com", "correct_password");
    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key("set-cookie"));
}
```

**Example — Rust line comment, nested capability:**

```rust
// @spec auth/google OAuth callback: Valid callback
#[test]
fn test_google_oauth_callback_happy_path() {
    let response = google_callback("valid_code");
    assert_eq!(response.status(), 302);
}
```

**Example — scenario name with colons:**

```rust
/// @spec api/users Create user: Email validation: rejects bad formats
#[test]
fn test_create_user_rejects_bad_email() {
    // ...
}
```

The first colon after the requirement name `Create user` separates
the requirement from the scenario. The remaining colons are part of
the scenario name `Email validation: rejects bad formats`.

Backlinks can appear in any source language supported by the
scanner. Support for additional languages is added by teaching the
scanner about that language's comment syntax. The format of the
backlink itself is language-agnostic.

## State derivation

duckspec stores no metadata in frontmatter, sidecar files, or
hidden state. All state is derived from the filesystem and file
contents. The derivation rules are:

| Fact                        | Source                                                               |
|-----------------------------|----------------------------------------------------------------------|
| Project constitution exists | File present at `duckspec/project.md`                                |
| Project name                | H1 of `project.md`                                                   |
| Capability exists           | Directory under `caps/` that contains `spec.md` (and after archive, also `doc.md`) |
| Capability path             | Directory's relative path from `caps/`                               |
| Capability title            | H1 of the capability's `spec.md`                                     |
| Capability summary          | Paragraph between H1 and next heading in `spec.md`                   |
| Grouping directory          | Directory under `caps/` with no `spec.md`                            |
| Codex entry exists          | Markdown file present under `codex/`                                 |
| Codex entry path            | File's relative path from `codex/`                                   |
| Codex entry title           | H1 of the codex entry file                                           |
| File is a delta             | Filename ends with `.delta.md` (only meaningful under `caps/`)       |
| File is full                | Filename ends with `.md` but not `.delta.md`                         |
| Active changes              | Subdirectories of `changes/`                                         |
| Change name                 | Folder name under `changes/`                                         |
| Change has a proposal       | `changes/<name>/proposal.md` exists                                  |
| Change has a design         | `changes/<name>/design.md` exists                                    |
| Change has capability work  | `changes/<name>/caps/` contains any `spec.md`, `spec.delta.md`, `doc.md`, or `doc.delta.md` |
| Change has steps            | `changes/<name>/steps/` contains `NN-<slug>.md` files                |
| Step name                   | H1 of the step file                                                  |
| Step slug                   | Filename after the `NN-` prefix, minus `.md`                         |
| Step order                  | Numeric `NN` prefix of the filename                                  |
| Step status                 | All task checkboxes in `## Tasks` checked                            |
| Current step                | Lowest-`NN` step with any unchecked tasks                            |
| Change phase                | Derived from which artifacts exist (see below)                       |
| Archived changes            | Subdirectories of `archive/`                                         |
| Archive date and counter    | `YYYY-MM-DD-NN` prefix of archive folder name                        |
| Archive change name         | Portion of archive folder name after `NN-`                           |

**Phase derivation.** The phase of a change is determined by which
artifacts exist in the change folder:

- **Proposal phase**: the change has only `proposal.md` and
  optionally `design.md`.
- **Spec phase**: the change has `caps/` entries but no `steps/`
  folder, or an empty `steps/` folder.
- **Step phase**: the change has `steps/` with at least one step
  file and at least one unchecked task somewhere.
- **Apply-ready**: the change has `steps/` and all tasks in all
  steps are checked.
- **Archived**: the change folder no longer exists under `changes/`
  but does exist under `archive/`.

A doc-only change (only `caps/*/doc.md` or `caps/*/doc.delta.md`
files, no spec work, no steps) is in spec phase until archived. A
proposal-only change is in proposal phase until archived.

## CLI commands

The `ds` binary provides the commands below. Commands are read-only
unless explicitly marked as mutating. Mutating commands write to
the `duckspec/` directory or source files.

### `ds init [<harness>]`

Creates the `duckspec/` directory structure in the current project.
Idempotent: can be run multiple times safely.

If a `<harness>` argument is provided, also installs agent command
template files for that harness into the appropriate location (for
example, `.claude/commands/` for `claude`, `.opencode/commands/`
for `opencode`). Supported harnesses are `claude` and `opencode`.
Running `ds init <harness>` multiple times with different harnesses
installs the templates for each.

Running with no `<harness>` argument only ensures the `duckspec/`
directory exists and contains its expected subdirectories
(`archive/`, `caps/`, `codex/`, `changes/`).

Mutating: writes to `duckspec/` and optionally to the harness
command directory.

### `ds status`

Prints a summary of the current duckspec state: active changes and
their phases, capability counts, codex counts, and the most recent
archive. The exact output format is implementation-defined.

Read-only.

### `ds audit`

Validates the whole project: the integrity of the code, the tests,
the backlinks that connect tests to scenarios, and the consistency
between the duckspec artifacts and the source code. This is the
"is the project in a good state" command.

Checks performed by `ds audit` include every `@spec` backlink in
source code resolving to an existing scenario, every `test: code`
scenario having at least one resolved backlink, every scenario
marked `test: code` in an active change being covered by a step
task, and any other cross-artifact integrity checks that require
global context.

For validation scoped to individual duckspec artifacts (single
files or paths inside `duckspec/`), use `ds check`.

`ds audit` is read-only: it does not modify files.

Read-only.

### `ds check [<path>] [--format]`

Validates duckspec artifacts against their schemas and structural
rules. The `<path>` argument selects what to check:

- A single file: validates just that file against the rules
  appropriate to its location (capability spec, capability doc,
  codex entry, `project.md`, proposal, design, step, spec delta,
  or doc delta).
- A directory under `duckspec/`: recursively validates every file
  within that directory.
- Omitted: validates every file under `duckspec/`.

`ds check` focuses on **artifact-level** validation: is this file
structurally valid, does it match its schema, are its headers in
canonical order, are its markers correct, does a delta file's
target capability exist. It does not check the project's source
code or the integrity of `@spec` backlinks — those are the
responsibility of `ds audit`.

**`--format`** rewrites the file (or every file under the
directory) to canonical order, fixing ordering violations in place.
Formatting is limited to ordering: other schema violations are
reported but not auto-fixed. After formatting, the file is
validated again and any remaining violations are reported normally.

Examples:

```
ds check duckspec/caps/auth/spec.md
ds check duckspec/changes/add-google-auth/
ds check duckspec/changes/add-google-auth/caps/auth/spec.delta.md --format
ds check duckspec/codex/architecture.md
ds check
```

Exits with a non-zero code if any validation errors remain after
any requested formatting.

Read-only unless `--format` is passed. When `--format` is passed,
mutating.

### `ds sync [--dry]`

Scans the project's source files for `@spec` backlinks, resolves
each backlink to a scenario in the current capability specs, and
augments the scenario's `> test: code` blockquote with the file
paths of the linked source locations. Any existing paths under a
`test: code` blockquote are replaced by the current set.

`ds sync --dry` shows the changes that would be made without
writing them.

After `ds sync`, every scenario with a `test: code` marker must
have at least one resolved backlink. Scenarios without backlinks
are validation errors at audit time. Backlinks pointing to
scenarios that do not exist are also validation errors.

Mutating (unless `--dry` is passed): rewrites spec files to include
resolved paths.

### `ds archive <name> [--dry]`

Archives the change at `changes/<name>/`. The command:

1. Validates the change's contents.
2. Applies the change's capability modifications to the top-level
   `caps/` directory:
   - Full `spec.md` or `doc.md` files in the change are copied to
     the corresponding capability folder under top-level `caps/`,
     replacing any existing file with the same path.
   - `spec.delta.md` and `doc.delta.md` files in the change are
     applied to their target capability files under top-level
     `caps/`.
3. Moves the entire `changes/<name>/` folder to
   `archive/YYYY-MM-DD-NN-<name>/`, where `NN` is a per-day counter
   starting at `01`.
4. Re-validates the resulting top-level state.

The operation is atomic: if any validation fails at step 1 or step
4, the entire operation is rolled back and the filesystem is left
in the pre-archive state.

`ds archive` does not touch `codex/` or `project.md`. Those
artifacts are edited directly and never participate in the archive
lifecycle.

A change with only a proposal can be archived: the archive step
moves the folder without modifying the top-level directories.

**`--dry`** previews the archive without writing: validates the
change, then prints the list of capability additions and delta
merges that would be applied. Useful for reviewing a change before
archiving.

Mutating (unless `--dry` is passed): writes to `caps/` and
`archive/`; removes the change folder from `changes/`.

### `ds index [--caps] [--codex] [--project]`

Prints a tree of duckspec artifacts with their summaries inline.
Useful for getting a quick overview of what the project covers.

Without flags, `ds index` prints everything: the project
constitution (if present), the capability tree, and the codex tree.
The flags scope the output:

- `--caps` — only the capability tree.
- `--codex` — only the codex tree.
- `--project` — only the project constitution.

Flags may be combined. The exact output format is
implementation-defined.

Read-only.

### `ds template <name>`

Prints the embedded agent command template named `<name>` to
standard output. Templates are the markdown files that drive the
`/ds-*` agent workflow. Examples: `ds template ds-explore`, `ds
template ds-spec`, `ds template ds-codex`.

Read-only.

### `ds schema <name>`

Prints the embedded schema description named `<name>` to standard
output. Schema descriptions are format references loaded into agent
context to explain how to author specific artifact types.

Read-only.

## Agent templates

duckspec's workflow is driven by agent command templates: markdown
files loaded into an AI agent's context when the user invokes a
`/ds-*` command in their harness. Each template instructs the
agent on how to assist with a particular phase of the workflow,
and which CLI commands to call for structural validation.

Templates are embedded in the `ds` binary and installed to a
harness-specific location by `ds init <harness>`. For example,
running `ds init claude` installs templates to `.claude/commands/`;
running `ds init opencode` installs them to `.opencode/commands/`.

Each template is associated with one phase of the workflow or with
a side operation:

- `ds-explore` — gather context for any duckspec activity: inspect
  the current state, identify whether work is needed, decide what
  kind of change (or codex harvest) to perform next
- `ds-plan` — planning: help author `proposal.md` and `design.md`
  for the current change
- `ds-spec` — speccing: help author capability specs and docs,
  either full new files or deltas against existing capabilities
- `ds-step` — stepping: break the change's implementation work
  into sequential step files
- `ds-apply` — applying: implement the current step (the first
  step with unchecked tasks)
- `ds-archive` — archiving: run `ds archive <name>` and handle any
  validation issues
- `ds-verify` — verification: run phase-aware validation with the
  CLI and surface any issues
- `ds-codex` — codex authoring: draft new or updated codex entries
  from the current session context and write them directly to
  `duckspec/codex/`

Templates are idempotent: installing them multiple times overwrites
previous versions, so `ds init <harness>` can be used to update
templates to the latest embedded version.

## Validation rules

The following rules are enforced by `ds audit`. Violations are
reported with file path, location, and rule identifier. Any
violation causes `ds audit` to exit with a non-zero status.

**Directory structure:**

- The `duckspec/` directory exists at the project root.
- `duckspec/` contains `archive/`, `caps/`, `codex/`, and
  `changes/` subdirectories (though any of these may be empty).
- `duckspec/project.md`, if present, is a single file directly
  under `duckspec/`.

**Capability paths:**

- Capability paths (directories under `caps/` containing a
  `spec.md`) contain no whitespace in any segment.
- Capability path segments use kebab-case by convention; this is
  not enforced as a hard error.

**Capability folder structure:**

- Every directory under `caps/` that contains a `spec.md` is a
  capability and must also contain a `doc.md` after archive.
- Directories under `caps/` that contain neither a `spec.md` nor
  any descendant capabilities have no purpose and should be
  cleaned up; they are tolerated but flagged as warnings.
- A capability's `spec.md` and `doc.md` have identical H1 text.

**Full file structure (applies to capability specs, capability
docs, codex entries, `project.md`, proposals, designs, steps):**

- File begins with an H1 header.
- No content appears before the H1 (no preamble).
- The H1 is followed directly by a summary paragraph.
- The summary is non-empty, is a single paragraph, and contains no
  block elements (no lists, no blockquotes, no code blocks, no
  headings).
- Zero or more freeform description blocks may appear between the
  summary paragraph and the next heading. Description content is
  parseable markdown but otherwise unconstrained.

**Capability spec structure:**

- All H2 headers start with the literal prefix `Requirement: `.
- Under every `## Requirement:` header, all H3 headers start with
  the literal prefix `Scenario: `.
- No H4 or deeper headers appear anywhere in the spec.
- Every requirement has either normative prose in its body, at
  least one scenario, or both.
- Requirement names do not contain colons.

**Scenario structure:**

- A scenario's body is exactly one unordered list of GWT bullets,
  optionally followed by a single test marker blockquote. No other
  content is allowed inside a scenario.
- Every scenario contains at least one `**WHEN**` bullet and at
  least one `**THEN**` bullet.
- Scenario bullet keywords are one of `GIVEN`, `WHEN`, `THEN`,
  `AND`.
- Bullet keywords are uppercase and bold.

**Test marker:**

- Every scenario resolves to a test marker, either directly in the
  scenario's blockquote or inherited from its parent requirement.
- The test marker blockquote, when present, appears at the end of
  its containing requirement or scenario body. A blockquote that
  parses as a marker but appears before other body content is a
  validation error (`*.test_marker.misplaced`).
- Test marker prefixes are one of `test:`, `manual:`, or `skip:`.
- `test:` markers have a value of `code`. Any sub-bullets under a
  `test: code` marker are resolved file paths.
- `manual:` and `skip:` markers have a non-empty reason.
- Every `test: code` scenario has at least one resolved backlink
  after `ds sync`.

**Codex:**

- Every file under `codex/` that has a `.md` extension is a codex
  entry and must satisfy the full file structure rules (H1 and
  summary).
- Codex entry path segments contain no whitespace.
- No further structure is enforced inside the codex body.

**Project constitution:**

- If `duckspec/project.md` exists, it must satisfy the full file
  structure rules (H1 and summary).
- No further structure is enforced inside the body.

**Step structure:**

- Step files are located at `changes/<name>/steps/NN-<slug>.md`.
- `NN` is a two-digit zero-padded integer.
- The step file's H1, when slugified, equals the `<slug>` portion
  of the filename.
- The step contains a `## Tasks` section with at least one task.
- Tasks are nested at most one level deep.
- Prerequisite items starting with `@step ` reference a valid step
  in the same change (matching by slug).

**Task content:**

- A task whose entire content is an `@spec ...` backlink references
  a scenario that exists in the change's capability specs or in the
  top-level capability specs.
- Every scenario in the change's capability specs marked `test:
  code` has at least one scenario implementation task in some step
  of the change.

**Delta files:**

- Delta files are located only under `changes/<name>/caps/<path>/`
  as `spec.delta.md` or `doc.delta.md`.
- Every H1, H2, and H3 in a delta file carries an explicit marker
  from the set `+`, `-`, `~`, `=`, `@`.
- Each header name at a given level appears at most once in a
  delta file. For renames, the old name (in the `=` entry) and the
  new name (in any other entry) are distinct names and both may
  appear.
- A `+` marker targets a header that does not exist in the source.
- A `-`, `~`, `=`, or `@` marker targets a header that exists in
  the source.
- A `=` entry's first non-blank line after the header is the new
  name, on a single line. The entry contains no additional body
  paragraphs, child headers, or blockquotes.
- A source header is renamed at most once per delta. Rename chains
  (renaming a header that was just created by another rename in
  the same delta) are forbidden.
- After all renames are applied, every header name at each level
  is unique. A rename whose new name collides with an existing
  header at the same level is an error.
- Delta entries within each header level appear in canonical
  order: `=`, `-`, `~`, `@`, `+`. The same rule applies
  recursively inside each `@` anchor. Entries out of canonical
  order are a validation error.
- The `@` marker is not valid on H3 headings (scenarios have no
  children to anchor into).
- A `-` entry must have an empty body. Non-empty bodies on remove
  entries are a validation error.
- A delta file targets an existing top-level capability.
- After apply, the resulting file is valid against its target
  type's schema.

**Full-file replace exclusivity:**

- Within a single change, a given capability file
  (`caps/<path>/spec.md` or `caps/<path>/doc.md`) is carried by
  **either** a full file or a delta file, not both. A
  `changes/<name>/caps/<path>/spec.md` and a
  `changes/<name>/caps/<path>/spec.delta.md` coexisting in the
  same change is a validation error.

**Backlinks:**

- Every `@spec` backlink in source code resolves to an existing
  scenario (capability exists, requirement exists, scenario
  exists).
- Broken backlinks (pointing to nonexistent capabilities,
  requirements, or scenarios) are errors.

**Changes:**

- Change folder names (under `changes/`) contain no whitespace.
- Archive folder names match the pattern `YYYY-MM-DD-NN-<name>`.
- Changes do not contain `codex/` or `project.md` entries; such
  files inside a change folder are a validation error.

## Not yet specified

The following areas are deliberately left unspecified in this
reference while their design is still being researched. See
`references/bdd-research.md` for the ongoing research notes and
open questions.

- **`[NEEDS CLARIFICATION]` markers** for flagging unresolved
  ambiguity in specs, docs, and other artifacts.
- **`ds lint`** as a separate command from `ds check`, for semantic
  quality signals (RFC 2119 keyword presence, implementation-detail
  word lists, vague-hedge lists).
- **`/ds-discover` or Example Mapping phase** as a conversational
  protocol for generating requirements and scenarios from rough
  intent.
- **Flat `tasks.md` as an alternative to `steps/NN-<slug>.md`** for
  small changes that don't need multi-session stepping.
- **Scenario outlines / parameterized scenarios** with
  Gherkin-style `Examples:` tables.
- **Constitution enforcement gates** (`project.md` as a gating
  mechanism in addition to context).
- **Spec authoring ruleset** teaching agents what good requirements
  and scenarios look like (RFC 2119, behavior-not-implementation,
  vague-hedge avoidance, declarative phrasing).
- **Cross-capability link syntax** for explicit dependency
  declarations between capabilities.
- **`ds move <old-path> <new-path>`** for atomic capability
  renames with backlink updates.

These items are tracked as open questions and will land in this
reference as they are decided.
