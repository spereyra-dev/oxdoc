# Fuzz regression corpus

This directory contains minimized, non-sensitive inputs retained after a fuzz
finding is fixed. Add a case below a directory named for its fuzz target, for
example `regressions/xlsx_sheet/<sha256>`.

Do not add customer documents, credentials, personally identifiable information,
or raw crash artifacts. Minimize the input with `cargo fuzz tmin`, inspect it,
and replace document content with synthetic data before committing it. Record the
finding or advisory and the reproduction command in the fixing pull request.

The scheduled workflow supplies the matching target directory as an additional
corpus, so retained cases are exercised on every scheduled run. The generated
`corpus/` and `artifacts/` directories remain ignored.
