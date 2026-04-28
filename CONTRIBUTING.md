# Contributing

Thanks for contributing to `wysprflow`.

## Before you start

- Read [docs/setup.md](docs/setup.md).
- Keep changes scoped to the task you are working on.
- Open an issue before large feature work if the direction is unclear.

## Local setup

```bash
pnpm install
pnpm tauri:dev
```

Validation commands:

```bash
pnpm build
pnpm check:rust
pnpm check
```

## Development notes

- Frontend lives in `src/`.
- Rust/Tauri backend lives in `src-tauri/`.
- Debug macOS builds store secrets in a local app config file instead of Keychain.
- Packaged macOS builds use Keychain.

## Pull requests

- Keep PRs focused.
- Update docs when behavior, setup, or release steps change.
- Add or update tests when the risk justifies it.
- Fill out the PR template.

## Style

- Follow the existing code patterns in the repo.
- Prefer small, direct abstractions.
- Do not ship placeholder UI for unfinished backend behavior.

## Reporting bugs

Use the bug report template and include:

- macOS version
- app version
- whether this was `tauri dev` or a packaged build
- provider configuration
- relevant run log output
