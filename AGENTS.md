# Code Review Rules

## General
- Keep changes minimal and focused; no unrelated refactors.
- Prefer explicit, readable code over clever constructs.

## TypeScript
- Use const/let, never var.
- Prefer strict types; avoid `any`.
- No unused imports or variables.

## React / Next.js
- Use functional components with named exports.
- Keep server/client boundaries explicit.

## Dependencies
- Lockfile and manifest must stay in sync.
- Avoid unnecessary dependency additions.
