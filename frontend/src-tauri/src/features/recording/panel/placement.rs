//! Where the record panel sits (ui.md, Record pill): against the left or
//! right edge of the screen's work area with a [`INSET`] gap, at a height
//! the user chose by dragging. Pure geometry in logical points, so the
//! snapping is tested without a screen.

use deino_core::settings::{PanelEdge, PanelPosition};

/// The gap between the pill and the screen edge.
pub const INSET: f64 = 16.0;

/// Where the panel goes the first time: the right edge, a third down
/// (Granola's nub).
pub const DEFAULT_POSITION: PanelPosition = PanelPosition {
    edge: PanelEdge::Right,
    from_top: 333,
};

/// A rectangle in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Whether the point (`x`, `y`) is inside, `slop` points of margin included.
    pub fn contains(&self, (x, y): (f64, f64), slop: f64) -> bool {
        x >= self.x - slop
            && x <= self.x + self.width + slop
            && y >= self.y - slop
            && y <= self.y + self.height + slop
    }
}

/// The window's size and the pill's margin inside it (room for its shadow).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The window's width.
    pub width: f64,
    /// The window's height.
    pub height: f64,
    /// Transparent margin between the window's edge and the pill.
    pub margin: f64,
}

/// The window's top-left corner for `position` on the work area `screen`,
/// so the pill keeps [`INSET`] from the edge and stays on screen.
pub fn origin(position: PanelPosition, frame: Frame, screen: Rect) -> (f64, f64) {
    let x = match position.edge {
        PanelEdge::Left => screen.x + INSET - frame.margin,
        PanelEdge::Right => screen.x + screen.width - INSET - frame.width + frame.margin,
    };
    let travel = (screen.height - frame.height).max(0.0);
    let y = screen.y + travel * f64::from(position.from_top.min(1000)) / 1000.0;
    (x, clamp_top(y, frame, screen))
}

/// The position a window dropped with its top-left at `(x, y)` snaps to:
/// the nearer edge, and its height on the work area.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to 0..=1000 first"
)]
pub fn snap((x, y): (f64, f64), frame: Frame, screen: Rect) -> PanelPosition {
    let center = x + frame.width / 2.0;
    let edge = if center < screen.x + screen.width / 2.0 {
        PanelEdge::Left
    } else {
        PanelEdge::Right
    };
    let travel = (screen.height - frame.height).max(1.0);
    let top = clamp_top(y, frame, screen) - screen.y;
    let from_top = (top / travel * 1000.0).round().clamp(0.0, 1000.0);
    PanelPosition {
        edge,
        from_top: from_top as u16,
    }
}

/// `y` kept so the pill stays [`INSET`] inside the top and bottom.
fn clamp_top(y: f64, frame: Frame, screen: Rect) -> f64 {
    let lowest = screen.y + INSET - frame.margin;
    let highest = screen.y + screen.height - INSET - frame.height + frame.margin;
    y.min(highest).max(lowest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Frame = Frame {
        width: 400.0,
        height: 120.0,
        margin: 12.0,
    };

    const SCREEN: Rect = Rect {
        x: 0.0,
        y: 25.0,
        width: 1512.0,
        height: 920.0,
    };

    #[test]
    fn the_default_is_the_right_edge_a_third_down() {
        let (x, y) = origin(DEFAULT_POSITION, FRAME, SCREEN);
        assert!((x - (1512.0 - 16.0 - 400.0 + 12.0)).abs() < 1e-9);
        assert!((y - (25.0 + 800.0 * 0.333)).abs() < 1e-9);
    }

    #[test]
    fn a_drop_snaps_to_the_nearer_edge_and_keeps_its_height() {
        let left = snap((300.0, 425.0), FRAME, SCREEN);
        let right = snap((900.0, 25.0), FRAME, SCREEN);

        assert_eq!(left.edge, PanelEdge::Left);
        assert_eq!(left.from_top, 500);
        assert_eq!(right.edge, PanelEdge::Right);
        assert!((origin(left, FRAME, SCREEN).0 - 4.0).abs() < 1e-9);
    }

    #[test]
    fn a_drop_past_the_top_or_bottom_stays_inset_on_screen() {
        let above = snap((900.0, -300.0), FRAME, SCREEN);
        let below = snap((900.0, 5_000.0), FRAME, SCREEN);
        let (_, top) = origin(above, FRAME, SCREEN);
        let (_, bottom) = origin(below, FRAME, SCREEN);

        assert!((top - (25.0 + 16.0 - 12.0)).abs() < 1e-9);
        assert!((bottom - (25.0 + 920.0 - 16.0 - 120.0 + 12.0)).abs() < 1e-9);
    }

    #[test]
    fn a_point_is_inside_a_rect_with_slop() {
        let pill = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 36.0,
        };
        assert!(pill.contains((10.0, 10.0), 0.0));
        assert!(pill.contains((8.0, 47.0), 2.0));
        assert!(!pill.contains((200.0, 20.0), 2.0));
    }
}
