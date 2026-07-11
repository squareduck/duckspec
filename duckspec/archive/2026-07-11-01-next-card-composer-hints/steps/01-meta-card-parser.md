# Meta-card parser

Add duckboard-only line parser for `write` / `next` meta cards and trailing next actions;
unit-test every meta-cards scenario.

## Tasks

- [x] 1. Add `crates/duckboard/src/meta_card.rs` with `MetaCard`, `NextAction`,
         `parse_meta_cards`, and `trailing_next_actions` (no duckpond)

- [x] 2. Wire the module into `crates/duckboard/src/main.rs` (or crate root) so it
         compiles

- [x] 3. @spec chat/meta-cards Card recognition: Known-kind quote run yields a card with inclusive line range

- [x] 4. @spec chat/meta-cards Card recognition: Ordinary blockquote is not a meta card

- [x] 5. @spec chat/meta-cards Card recognition: Known-kind line inside a fenced code block is not a meta card

- [x] 6. @spec chat/meta-cards Trailing next actions: Trailing next card yields ordered send tokens

- [x] 7. @spec chat/meta-cards Trailing next actions: Non-trailing next card yields no actions

- [x] 8. @spec chat/meta-cards Trailing next actions: Actions capped at three in source order

- [x] 9. @spec chat/meta-cards Trailing next actions: Body line without a token is skipped

- [x] 10. @spec chat/meta-cards Trailing next actions: Reason after the token is not part of send text
