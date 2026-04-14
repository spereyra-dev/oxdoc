#![no_main]

use libfuzzer_sys::fuzz_target;
use oxdoc_core::fuzz_relationships;

fuzz_target!(|data: &[u8]| {
    let _ = fuzz_relationships(data);
});
