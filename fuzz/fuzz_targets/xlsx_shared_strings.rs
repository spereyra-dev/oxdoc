#![no_main]

use libfuzzer_sys::fuzz_target;
use oxdoc_core::fuzz_parse_shared_strings;

fuzz_target!(|data: &[u8]| {
    let _ = fuzz_parse_shared_strings(data);
});
