//! Lecture du texte d'une page surveillée, avec et sans zones. Première ligne :
//! sélecteurs à lire ; deuxième : sélecteurs à écarter ; le reste : la page.
#![no_main]

use dumbmonit_server::webchange::html;
use dumbmonit_server::webchange::zones::{Selector, Zones};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let mut lines = input.splitn(3, '\n');
    let content = lines.next().unwrap_or_default();
    let exclude = lines.next().unwrap_or_default();
    let page = lines.next().unwrap_or_default();
    let zones = Zones {
        content: Selector::parse_list(content).unwrap_or_default(),
        exclude: Selector::parse_list(exclude).unwrap_or_default(),
        drop_chrome: true,
    };
    let _ = html::parse_with(page, &zones);
    let _ = html::parse(input);
});
