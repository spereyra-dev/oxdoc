#![no_main]

use libfuzzer_sys::fuzz_target;
use oxdoc_core::fuzz_metadata;

fuzz_target!(|data: &[u8]| {
    let _ = fuzz_metadata(data);
});
