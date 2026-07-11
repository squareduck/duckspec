# Priming segment flag and collapse policy

Carry `is_priming` from messages through transcript segments and blocks; default and sync
collapse so Setup starts folded and normal users stay open.

## Tasks

- [x] 1. Add `is_priming` on `TranscriptSeg::User` and `text_edit::Block`; set from
         `ChatMessage::is_priming` in `build_transcript_segments` / `blocks_from_segments`
         with Setup label when priming

- [x] 2. First-sight and `sync_collapse_states`: priming User starts/stays collapsed when
         not `user_set`; non-priming User/Answer/System stay open

- [x] 3. @spec chat/transcript Collapse defaults: Priming Setup starts collapsed

- [x] 4. @spec chat/transcript Collapse defaults: User-expanded priming is not force-collapsed by sync
