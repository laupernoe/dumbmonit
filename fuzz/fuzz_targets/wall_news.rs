//! Flux RSS 2.0, RSS 1.0 et Atom du bandeau d'actualités du mur.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = dumbmonit_server::api::fuzz_wall_news(input);
});
