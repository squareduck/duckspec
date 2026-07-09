# Extract shared attach walk

Pull Claude's `[label](attach:<id>)` walker into a crate-private `attach` module with
neutral `Segment`s so both harnesses share parse rules; Claude keeps Anthropic encoding
over the shared walk.

## Tasks

- [x] 1. Add `crates/duckchat/src/attach.rs` with `Segment::{Text, Image}` and
         `walk(prompt, attachments) -> Vec<Segment>` (image/* → Image, non-image → text
         fallback, unresolved/malformed → literal text)

- [x] 2. Declare `mod attach;` in `crates/duckchat/src/lib.rs` (crate-private; not
         re-exported)

- [x] 3. Refactor `assemble_user_content` in `crates/duckchat/src/claude_code/run.rs` to
         call `attach::walk` and encode Anthropic content blocks (`source.media_type` /
         base64) from segments

- [x] 4. Move the walk-focused unit tests from `claude_code/run.rs` onto `attach::walk`
         (plain text, one image, interleaved, unresolved, non-attach link, malformed,
         empty); leave Anthropic encode assertions on the Claude path if still useful

- [x] 5. Run duckchat tests covering the Claude attach path and confirm they stay green
