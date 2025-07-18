pub mod core;
pub mod filesystem;
pub mod legacy_pattern;
pub mod list;
pub mod math;
pub mod pattern;
pub mod pattern_test;
pub mod text;
pub mod time;
pub mod typechecker;

use crate::interpreter::environment::Environment;

pub fn register_stdlib(env: &mut Environment) {
    core::register_core(env);
    filesystem::register_filesystem(env);
    math::register_math(env);
    text::register_text(env);
    list::register_list(env);
    pattern::register(env);
    time::register_time(env);
}
