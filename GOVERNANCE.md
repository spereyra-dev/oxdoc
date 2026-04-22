# Governance

`oxdoc` starts as a maintainer-led open source project.

## Maintainer Responsibilities

Maintainers are responsible for:

- Setting project scope.
- Reviewing and merging pull requests.
- Cutting releases.
- Responding to security reports.
- Keeping the roadmap realistic.

## Decision Making

Small implementation decisions can be made in pull requests. Larger design decisions should be discussed in an issue before implementation.

When there is disagreement, maintainers make the final call based on project goals: fast extraction, bounded memory, predictable output, and a small public API.

## Branch Protection and Merging

The default branch, `main`, is protected. Normal project work lands through pull requests after required checks pass:

- `rust`
- `validate`

External contributions require maintainer review. Review conversations should be resolved before merge. Direct pushes to `main` are not part of the normal workflow.

Emergency maintainer bypasses should be rare, documented publicly in an issue or follow-up PR, and verified with the same local checks expected from any other change.

## Becoming a Maintainer

Additional maintainers may be added after sustained, high-quality contributions and demonstrated care for review quality, compatibility, and project scope.
