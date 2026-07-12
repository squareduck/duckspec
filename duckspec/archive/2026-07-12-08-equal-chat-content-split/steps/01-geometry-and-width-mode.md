# Geometry and width mode

Pure free-space helpers, `width_customized` + rebalance/lock behavior, and unit tests for
equal-width and grip-customization scenarios.

## Tasks

- [x] 1. Add pure helpers `free_content_chat_width` and `equal_interaction_width` (export
         handle width / min panel width as needed) colocated with three-column chrome
         constants

- [x] 2. Add `width_customized: bool` to `InteractionState` (default false); seed default
         `width` from `equal_interaction_width` for the initial window size

- [x] 3. On `HandleMsg::SetWidth`, set width and mark `width_customized = true`; leave the
         flag alone for toggle and content collapse

- [x] 4. Add `rebalance_uncustomized(ix, window_w)` that sets equal width only when not
         customized

- [x] 5. @spec layout/content-chat-split Uncustomized equal width: Default half of free space

- [x] 6. @spec layout/content-chat-split Uncustomized equal width: Resize rebalances to half free space

- [x] 7. @spec layout/content-chat-split Uncustomized equal width: Half floors at minimum panel width

- [x] 8. @spec layout/content-chat-split Uncustomized equal width: Half may exceed the old fixed max width

- [x] 9. @spec layout/content-chat-split Grip customization: First grip width change locks absolute width

- [x] 10. @spec layout/content-chat-split Grip customization: Resize after lock keeps absolute width

- [x] 11. @spec layout/content-chat-split Grip customization: Open/close and content collapse do not lock
