//! Profils SNMP en YAML, livrés ou importés par un administrateur (paquets).
#![no_main]

use dumbmonit_collectors::snmp::Profile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = Profile::parse(input);
});
