//! The optimization presets on [`OptimizationOptions`].

use frost_compile::{Optimization, OptimizationOptions};

#[test]
fn none_turns_every_optimization_off() {
    for &optimization in Optimization::ALL {
        assert!(
            !OptimizationOptions::NONE.get(optimization),
            "{optimization:?} is off"
        );
    }
}

#[test]
fn all_turns_every_optimization_on() {
    for &optimization in Optimization::ALL {
        assert!(
            OptimizationOptions::ALL.get(optimization),
            "{optimization:?} is on"
        );
    }
}
