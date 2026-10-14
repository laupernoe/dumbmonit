//! Sélecteurs de zones d'une page surveillée (`content`, `exclude`), tels
//! qu'un utilisateur les saisit.
#![no_main]

use dumbmonit_server::webchange::zones::Selector;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = Selector::parse_list(input);
});
