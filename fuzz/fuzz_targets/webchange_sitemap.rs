//! `sitemap.xml` et `robots.txt` d'un site surveillé.
#![no_main]

use dumbmonit_server::webchange::{robots, sitemap};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = sitemap::parse(input);
    let _ = robots::Robots::parse(input);
});
