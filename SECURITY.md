# Security Policy

`oxdoc` parses untrusted Office files, so parser safety is part of the core project scope.

## Supported Versions

Security fixes are made on the default branch and released as patched versions. After 1.0, supported release lines follow semantic versioning.

## Reporting a Vulnerability

Do not open public issues for security vulnerabilities.

Use GitHub private vulnerability reporting or contact the maintainers privately through GitHub. Include:

- A description of the issue and impact.
- A minimal reproducer, if safe to share.
- The affected file type and command.
- Whether the issue causes panic, denial of service, incorrect output, or data exposure.

Maintainers will acknowledge valid reports and coordinate a fix before public disclosure.

## Dependency Advisories

Dependency advisories are checked with `cargo audit` against the RustSec Advisory Database.

- Pull requests and pushes to `main` run the `security` workflow when Cargo dependency files, security docs, or the workflow change.
- A scheduled `security` workflow runs every Monday to catch newly published advisories against the existing `Cargo.lock`.
- Maintainers can run the same check locally with `make audit` after installing `cargo-audit`.

If the check reports a vulnerable direct or transitive dependency:

1. Prefer updating to a patched version with `cargo update -p <crate>` or by adjusting the direct dependency that pulls it in.
2. Confirm the advisory impact against `oxdoc`'s reachable parser, CLI, and library code paths.
3. Run `make audit` and the normal CI gate before merging the fix.
4. If no patched version exists, document the temporary risk decision in the PR and keep the issue open until the advisory is resolved or the dependency is removed.

Dependabot is configured for weekly Cargo and GitHub Actions updates. Those PRs should be treated as part of the security maintenance flow and should still pass the RustSec advisory scan before merge.

The maintainer process is documented in [docs/security.md](docs/security.md).

## Parser Safety Expectations

- Malformed ZIP/XML input should return errors or warnings, not panic.
- Fuzzing targets should be added for high-risk parser paths.
- Large inputs should not require loading the full document into memory unless explicitly documented.
