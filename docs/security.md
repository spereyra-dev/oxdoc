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
