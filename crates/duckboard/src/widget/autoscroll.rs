//! Shared edge auto-scroll velocity kernel.
//!
//! When a drag-selection runs past the edge of a viewport, the view should keep
//! scrolling on its own — faster the further the pointer overshoots — until the
//! pointer returns inside. This module is the one pure, independently-testable
//! piece of that behavior: it maps a pointer position against a viewport span to
//! a signed per-frame scroll velocity. Both the text editor and the terminal
//! consume it and convert the result to their own native scroll unit, so this is
//! also the single place to tune the ramp. It has zero coupling to anything else
//! in `duckboard`.

/// Per-frame velocity at the edge before ramp (logical px). Low → barely
/// crossing creeps.
const BASE: f32 = 1.0;
/// Extra per-frame velocity per logical px past the edge. Gentle ramp.
const RAMP: f32 = 0.12;
/// Upper bound so a fling to the far side stays controllable (~1200 px/s @60fps).
const MAX: f32 = 20.0;

/// Signed logical px/frame for a pointer at `pointer_y` against `[top, bottom]`.
/// `0.0` inside the span; positive = past bottom; negative = past top.
pub fn edge_velocity(pointer_y: f32, top: f32, bottom: f32) -> f32 {
    if pointer_y > bottom {
        let overshoot = pointer_y - bottom;
        (BASE + overshoot * RAMP).min(MAX)
    } else if pointer_y < top {
        let overshoot = top - pointer_y;
        -(BASE + overshoot * RAMP).min(MAX)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_inside_viewport_and_on_edges() {
        assert_eq!(edge_velocity(50.0, 0.0, 100.0), 0.0);
        // Exactly on each edge is still inside the span → no scroll.
        assert_eq!(edge_velocity(0.0, 0.0, 100.0), 0.0);
        assert_eq!(edge_velocity(100.0, 0.0, 100.0), 0.0);
    }

    #[test]
    fn positive_just_past_bottom() {
        // 1 px past the bottom: BASE + 1 * RAMP.
        assert_eq!(edge_velocity(101.0, 0.0, 100.0), BASE + RAMP);
        assert!(edge_velocity(101.0, 0.0, 100.0) > 0.0);
    }

    #[test]
    fn negative_just_past_top() {
        // 1 px past the top: -(BASE + 1 * RAMP).
        assert_eq!(edge_velocity(-1.0, 0.0, 100.0), -(BASE + RAMP));
        assert!(edge_velocity(-1.0, 0.0, 100.0) < 0.0);
    }

    #[test]
    fn ramps_with_distance() {
        let near = edge_velocity(110.0, 0.0, 100.0);
        let far = edge_velocity(120.0, 0.0, 100.0);
        assert!(far > near);
        assert_eq!(near, BASE + 10.0 * RAMP);
        assert_eq!(far, BASE + 20.0 * RAMP);
    }

    #[test]
    fn clamps_to_max_both_directions() {
        // A huge overshoot is bounded by MAX in either direction.
        assert_eq!(edge_velocity(10_000.0, 0.0, 100.0), MAX);
        assert_eq!(edge_velocity(-10_000.0, 0.0, 100.0), -MAX);
    }

    #[test]
    fn symmetric_around_the_edges() {
        // Equal overshoot past either edge → equal magnitude, opposite sign.
        let past_bottom = edge_velocity(140.0, 0.0, 100.0);
        let past_top = edge_velocity(-40.0, 0.0, 100.0);
        assert_eq!(past_bottom, -past_top);
    }
}
