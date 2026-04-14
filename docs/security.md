# Security

`oxdoc` parses untrusted Office files, so parser safety is part of the core project scope.

## Report Privately

Do not open public issues for security vulnerabilities.

Use GitHub private vulnerability reporting or contact the maintainers privately through GitHub.

Include:

- A description of the issue and impact.
- A minimal reproducer, if safe to share.
- The affected file type and command.
- Whether the issue causes panic, denial of service, incorrect output, or data exposure.

## Dependency Advisory Automation

`oxdoc` uses `cargo audit` for RustSec advisory checks. The `security` GitHub Actions workflow runs on relevant pull requests, pushes to `main`, a weekly schedule, and manual dispatch.

Run the same check locally:

```sh
cargo install cargo-audit --locked
make audit
```

The check reads `Cargo.lock` and fails when a dependency is affected by a RustSec advisory. Scheduled runs are important because a clean lockfile can become vulnerable after a new advisory is published.

## Dependency Update Flow

Dependabot is configured in `.github/dependabot.yml` for weekly Cargo dependency updates and GitHub Actions updates. Maintainers should review those PRs as normal code changes:

- Check whether the update fixes a RustSec advisory, a yanked crate, or an Actions security issue.
- Read release notes for parser-relevant dependencies before merging.
- Run `make audit` and the regular CI gate locally when the update changes dependency behavior or generated lockfile content.
- Keep the advisory issue or PR open when no patched dependency is available.

When `cargo audit` reports a vulnerable transitive dependency:

1. Prefer `cargo update -p <crate>` if the existing dependency constraints allow a patched version.
2. If the crate is pulled in transitively, update the direct dependency that owns the edge or replace the dependency when a fix is not available.
3. Confirm whether the vulnerable code is reachable from `oxdoc`'s supported DOCX, XLSX, metadata, CLI, or library workflows.
4. Document any temporary ignore or risk acceptance in the PR, including the advisory ID and the reason it is safe for now.

## Expectations

- Malformed ZIP/XML input should return errors or warnings, not panic.
- Required encrypted ZIP parts fail with `UnsupportedEncryptedPart`; password-protected Office documents are not decrypted.
- Required ZIP parts fail before reading when their uncompressed size exceeds 64 MiB.
- Required ZIP parts fail as suspicious when they are at least 4 MiB and their uncompressed-to-compressed ratio exceeds 200:1.
- Relationship targets must stay inside the OOXML package root and must not use external URLs, URI schemes, Windows drive prefixes, NUL bytes, or backslashes.
- Fuzzing targets should be added for high-risk parser paths.
- Large inputs should not require loading the full document into memory unless explicitly documented.
- Sensitive sample files should not be attached to public issues.

## Supported Versions

The project is pre-1.0. Security fixes are made on the default branch until versioned releases are established.
