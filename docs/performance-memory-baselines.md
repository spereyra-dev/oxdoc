# Peak Memory Baselines

Generated: 2026-05-15 12:44:49 UTC

- Platform: `macOS-26.4.1-arm64-arm-64bit`
- Machine: `arm64`
- Python: `3.12.9`
- Rust: `rustc 1.90.0 (1159e78c4 2025-09-14)`
- Binary: `target/release/oxdoc`
- Samples per case: `1`

| Case | Fixture | Output | Peak RSS MiB | Notes |
| --- | ---: | ---: | ---: | --- |
| `docx-text-256kb` | 421.2 KiB | 256.0 KiB | 3.1 | DOCX text extraction, 256 KiB extracted text |
| `pptx-text-256kb` | 393.3 KiB | 256.2 KiB | 3.6 | PPTX slide text extraction, 256 slides |
| `xlsx-shared-strings-spill` | 8.2 MiB | 8.0 MiB | 10.8 | XLSX shared-string-heavy CSV extraction past spill threshold |
| `xlsx-wide-sparse` | 14.0 KiB | 138.1 KiB | 2.7 | XLSX wide/sparse row CSV extraction |

Regenerate with:

```bash
python3 scripts/peak-memory-baselines.py --iterations 3 --output docs/performance-memory-baselines.md
```
