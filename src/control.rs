/// Different behaviors for the crate to allow overriding the colored output
/// behaviors. Creating the environment variable `NO_COLOR` (value is not
/// relevant) will disable all coloring. There is also some detection going on
/// to decide what kind of terminal type is used and if coloring should be used
/// or not. See [`colored::control`] for more information.
#[derive(Debug)]
pub enum ColoringMode {
    /// Output will always use color regardless of environment variable or
    /// terminal type.
    AlwaysColor,

    /// Output will never use color regardless of environment variable or
    /// terminal type.
    NeverColor,

    /// Set library to automatically detect if output should use color or not.
    UseEnvironment,
}

/// Change coloring mode across the library. See [`ColoringMode`] for more
/// information. By default the library will detect if the output should use
/// color or not [`ColoringMode::UseEnvironment`].
pub fn set_coloring_mode(control: &ColoringMode) {
    match control {
        ColoringMode::AlwaysColor => always_color(),
        ColoringMode::NeverColor => never_color(),
        ColoringMode::UseEnvironment => use_environment(),
    }
}

/// Set coloring mode to always use color in the output
/// ([`ColoringMode::AlwaysColor`]).
pub fn never_color() {
    colored::control::set_override(false);
}

/// Set coloring mode to never use color in the output
/// ([`ColoringMode::NeverColor`]).
pub fn always_color() {
    colored::control::set_override(true);
}

/// Set coloring mode detect if color should be used in the output or not
/// ([`ColoringMode::UseEnvironment`]).
pub fn use_environment() {
    colored::control::unset_override();
}
