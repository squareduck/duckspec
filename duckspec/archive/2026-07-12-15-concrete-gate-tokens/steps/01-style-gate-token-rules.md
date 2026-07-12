# Style gate token rules

Encode decision-named tokens and the reason split in the shared style schema so every
template can follow one rule.

## Tasks

- [x] 1. In `crates/duckspec/content/schemas/style.md`, update the `next` meta card rules:
         send tokens name the decision; bare `confirm` / `reject` are not the stock
         pattern

- [x] 2. Document the reason split: omit reasons on decision tokens; keep short UI reasons
         on slash-command handoffs only

- [x] 3. Replace Write gate and handoff examples with concrete tokens (e.g.
         `confirm proposal` / `reject proposal`) and slash lines that retain reasons
