pub mod cli;
pub mod core;
pub mod fs;
pub mod legacy_pattern;
pub mod list;
pub mod math;
pub mod path;
pub mod pattern;
pub mod pattern_test;
pub mod text;
pub mod time;
pub mod typechecker;

use crate::interpreter::environment::Environment;

pub fn register_stdlib(env: &mut Environment) {
    cli::register_cli(env);
    core::register_core(env);
    fs::register_fs(env);
    math::register_math(env);
    path::register_path(env);
    text::register_text(env);
    list::register_list(env);
    pattern::register(env);
    time::register_time(env);
}
