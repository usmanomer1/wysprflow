# Local Setup

## Prerequisites

- `macOS 13+`
- Xcode command line tools
- `pnpm`
- Rust stable

Install common prerequisites:

```bash
xcode-select --install
brew install pnpm
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Install dependencies

```bash
pnpm install
```

## Run the app

```bash
pnpm tauri:dev
```

That starts the Vite frontend and the Tauri desktop shell together.

## Validation commands

```bash
pnpm build
pnpm check:rust
pnpm check
```

## Secrets in development

On macOS debug builds, provider keys are stored in a local app config file instead of Keychain. This avoids repeated Keychain password prompts from an unsigned dev binary.

Packaged macOS builds use Keychain.

## Where things live

- `src/` - React frontend
- `src/routes/` - app screens
- `src/components/` - shared UI and settings sections
- `src-tauri/src/` - Rust backend
- `src-tauri/icons/` - bundled app icons

## Common contributor loop

1. Run `pnpm tauri:dev`
2. Make the change
3. Validate with `pnpm check`
4. Update docs if setup, release, or provider behavior changed
