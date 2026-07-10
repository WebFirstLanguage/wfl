pub mod cache;
pub mod core;
pub mod crypto;
pub mod filesystem;
pub mod helpers;
pub mod json;
pub mod list;
pub mod math;
pub mod media;
pub mod pattern;
pub mod pattern_test;
pub mod random;
pub mod text;
pub mod time;
pub mod typechecker;
pub mod web;

use crate::interpreter::environment::Environment;

pub fn register_stdlib(env: &mut Environment) {
    core::register_core(env);
    crypto::register_crypto(env);
    filesystem::register_filesystem(env);
    json::register_json(env);
    math::register_math(env);
    random::register_random(env);
    text::register_text(env);
    list::register_list(env);
    pattern::register(env);
    time::register_time(env);
    web::register_web(env);
    media::register_media(env);
    cache::register_cache(env);
}
