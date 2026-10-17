use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::vec2f;

use super::bounds_overlap_any_monitor;

fn rect(x: f32, y: f32, width: f32, height: f32) -> RectF {
    RectF::new(vec2f(x, y), vec2f(width, height))
}

#[test]
fn off_screen_window_does_not_count_as_visible() {
    let monitor = rect(0., 0., 1920., 1080.);
    assert!(bounds_overlap_any_monitor(
        rect(100., 100., 800., 600.),
        &[monitor]
    ));
    assert!(bounds_overlap_any_monitor(
        rect(1800., 0., 800., 600.),
        &[monitor]
    ));
    assert!(!bounds_overlap_any_monitor(
        rect(1920., 0., 800., 600.),
        &[monitor]
    ));
    assert!(!bounds_overlap_any_monitor(
        rect(-2000., 0., 800., 600.),
        &[monitor]
    ));
    assert!(!bounds_overlap_any_monitor(rect(0., 0., 800., 600.), &[]));
}
