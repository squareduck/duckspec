# Velocity kernel

Add the shared, pure `edge_velocity()` ramp/clamp function that both the text editor and
the terminal will consume, with its own unit tests.

## Tasks

- [x] 1. Create `crates/duckboard/src/widget/autoscroll.rs` with a module doc comment
         describing the shared edge auto-scroll purpose.

- [x] 2. Add the `BASE = 1.0`, `RAMP = 0.12`, `MAX = 20.0` constants with the doc comments
         from the design (carried over unchanged — re-tuning is out of scope).

- [x] 3. Implement `pub fn edge_velocity(pointer_y: f32, top: f32, bottom: f32) -> f32`:
         return `0.0` inside `[top, bottom]`, otherwise a signed velocity that ramps
         linearly with overshoot distance (`BASE + overshoot * RAMP`) and clamps to `MAX`;
         positive past the bottom, negative past the top.

- [x] 4. Add inline `#[cfg(test)] mod tests` covering: zero inside viewport (and exactly
         on each edge), positive just past bottom, negative just past top, ramps with
         distance, clamps to `MAX` both directions, symmetry around the edges.

- [x] 5. Register the module: add `pub mod autoscroll;` to
         `crates/duckboard/src/widget.rs`.
