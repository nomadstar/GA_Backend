use quickshift::algorithm::extract_controller::{set_use_optimized, is_using_optimized};

#[test]
fn test_controller_dispatches_to_optimized() {
    let old = is_using_optimized();
    set_use_optimized(true);
    assert!(is_using_optimized(), "El flag debe estar activado");
    set_use_optimized(old);
}

#[test]
fn test_controller_can_switch_to_original() {
    let old = is_using_optimized();
    set_use_optimized(false);
    assert!(!is_using_optimized(), "El flag debe estar desactivado");
    set_use_optimized(old); // restore
}
