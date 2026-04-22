# FAQ

## Is `oxdoc` a renderer?

No. It extracts data. It does not render documents, generate PDFs, preserve layout, or calculate pagination.

## Why not implement the full OOXML specification?

The full OOXML surface is large and includes many rendering and layout concerns that are outside the project scope. `oxdoc` focuses on fast extraction paths that are useful in automation.

## Does it support PPTX?

Yes. `oxdoc extract text deck.pptx` extracts slide text boxes in presentation order and includes linked speaker notes. It does not render slides, synthesize bullets, or preserve layout.

## Is memory bounded for huge XLSX files?

Shared-string tables use bounded memory within documented ZIP input limits: values stay in memory up to an internal threshold, then spill to temporary files. Worksheet rows are still buffered one row at a time, so very wide or extremely sparse rows can allocate more memory before they are written.

## Where do warnings go?

The CLI writes warnings to stderr. The library returns warnings in `Extraction<T>`.

## Can I attach a failing Office document to an issue?

Only if it is safe to redistribute. Do not attach private, confidential, or customer documents. Prefer a minimal reproduction file.

## Why Docsify?

Docsify keeps the documentation source as Markdown and serves it as a static documentation site without a build step. That matches the project's current size and keeps contribution friction low.
