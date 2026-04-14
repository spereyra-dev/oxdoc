# Security Policy

`oxdoc` parses untrusted Office files, so parser safety is part of the core project scope.

## Supported Versions

The project is pre-1.0. Security fixes are made on the default branch until versioned releases are established.

## Reporting a Vulnerability

Do not open public issues for security vulnerabilities.

Use GitHub private vulnerability reporting or contact the maintainers privately through GitHub. Include:

- A description of the issue and impact.
- A minimal reproducer, if safe to share.
- The affected file type and command.
- Whether the issue causes panic, denial of service, incorrect output, or data exposure.

Maintainers will acknowledge valid reports and coordinate a fix before public disclosure.

## Parser Safety Expectations

- Malformed ZIP/XML input should return errors or warnings, not panic.
- Fuzzing targets should be added for high-risk parser paths.
- Large inputs should not require loading the full document into memory unless explicitly documented.

