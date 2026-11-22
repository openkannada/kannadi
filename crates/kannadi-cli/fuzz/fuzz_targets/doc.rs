#![no_main]
#![allow(dead_code)]

#[path = "shared.rs"]
mod shared;
use shared::*;

libfuzzer_sys::fuzz_target!(|bytes: &[u8]| {
    if let Ok(document) = doc::read(bytes) {
        write(&document);
    }
});
