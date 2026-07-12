# Post-implementation review: archive list UX

Post-implementation pass on `archive-list-ux` (including followup section counts). Intent
is met; two minor craft/UX nits; archive-ready if those are deferred.

## Scope

Proposal, design, `exploration/archive` + `archive/browse`, steps 01–04, followup
`01-followup-change-list-section-counts`, and code in `chat_store.rs`, `data.rs`,
`area/change.rs`, `area/dashboard.rs`, `area/ideas.rs`. Stage: implemented + audit-clean.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Archive hides selected exploration in collapsed section | /ds-step |
| 2 | minor | fidelity | Same-day interleave keys not normalized | ignore |
```

## Findings

### 1. Archive hides selected exploration in collapsed section - quality/minor

**Where:** `crates/duckboard/src/area/change.rs` — `Message::ArchiveExploration`

**Why:** Soft-archive keeps selection (by design) but does not expand `"archived"`. The
row leaves the open Change list and lands under the default-collapsed Archived section, so
the active exploration disappears from the list while still selected. `SelectChange`
already expands Archived when navigating to an archived item; archive should match that
reveal path.

**Action:** On archive, if the archived id is selected (or always after archive of current
selection), `expanded_sections.insert("archived")`. Optional `/ds-step` polish; safe to
ignore if accepting the gap.

### 2. Same-day interleave keys not normalized - fidelity/minor

**Where:** `ArchivedEntry::sort_key` in `area/change.rs`; design Risk on ISO vs
`YYYY-MM-DD-NN`

**Why:** Design called for comparable keys; code compares full archive folder name to raw
`archived_at`. Lexicographically, `T` ranks above `-` at the date boundary, so on a given
day every exploration stamp sorts above every change counter. Order is still roughly
reverse-chronological across days; same-day mix is slightly biased. Low lasting harm.

**Action:** `ignore` unless same-day precision matters; if fixed later, normalize to
`(date, time|NN)` before sort.

## Verdict

**Ship-ready.** Proposal intent is realized: newest-first archives, soft-archive + hover
archive/remove, interleaved Dashboard/Change Archived, Ideas/Change Archive collapsed by
default, section counts on Change/Archived. Caps and tests align with behavior; no
soundness blockers. Findings are optional polish, not freeze risks.
