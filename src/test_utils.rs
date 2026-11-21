//! Helpers shared by the unit tests.
use std::sync::Mutex;
use std::sync::MutexGuard;

/// Serialize the tests depending on the global color override of the
/// `colored` crate.
static COLORS: Mutex<()> = Mutex::new(());

/// Force colored output on or off until the returned guard is dropped.
fn set_colors(enabled: bool) -> MutexGuard<'static, ()> {
    // A test failing while holding the lock poisons it, other tests can
    // still run.
    let guard = COLORS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    colored::control::set_override(enabled);
    guard
}

/// Disable colored output until the returned guard is dropped.
pub fn no_colors() -> MutexGuard<'static, ()> {
    set_colors(false)
}

/// Enable colored output until the returned guard is dropped.
pub fn colors() -> MutexGuard<'static, ()> {
    set_colors(true)
}
