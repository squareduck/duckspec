# Proposal schema

A proposal is the **pitch** for a change: **what** we want and **why** - before
design and before capability layout. It is not architecture, not a caps list, not
an impact analysis. Those come later (design, then specs).

## Structure

```markdown
# <Change Title>

<1-2 sentence summary>

## Motivation

<why this change, why now>

## Intent

<what should be true when this change succeeds - outcomes, behaviors, constraints
on the problem. Product/user language, not module or capability paths.>

## Non-goals

<what this change deliberately does not try to solve>
```

Recommended sections, not enforced by `ds check` beyond H1 + summary.

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- Body is freeform markdown; the Structure skeleton is the expected shape
- Path: `duckspec/changes/<change-name>/proposal.md`

## Quality

- **Motivation** answers why and why now - not a solution design.
- **Intent** is the success picture: what becomes true, for whom, under what
  constraints. Stay above capabilities and code. Naming exact `caps/` paths or
  listing files is premature here; design and spec discover structure.
- **Non-goals** bound the problem so later stages do not silently expand it.
  Feature-level, not “we will not touch crate X” unless that *is* the product
  boundary.
- Short and scannable. Persuasive pitch, not a mini-design.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# Add Google OAuth login

Let users sign in with Google so signup is not blocked on inventing a password.

## Motivation

Analytics show 40% of signup drop-offs happen at password creation. The largest
segment already has a Google account; meeting them there should recover that
funnel without weakening account security.

## Intent

- A new user can create a session with Google in one consent flow
- Returning Google users land in the same account they used before
- Password-based signup remains available; OAuth is an additional path
- Failure modes (denied consent, IdP outage) leave the user able to try again
  or use password signup

## Non-goals

- Apple or other IdPs in this change
- Linking an existing email/password account to Google from settings
- Changing session lifetime or auth factors beyond accepting a Google identity
```
