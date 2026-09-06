#![no_main]

use libfuzzer_sys::fuzz_target;
use oxdoc_core::fuzz_pptx_text;

fuzz_target!(|data: &[u8]| {
    let _ = fuzz_pptx_text(data);
});
