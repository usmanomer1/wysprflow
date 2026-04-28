# Install on macOS

This section is for normal users, not developers.

## Download

When the project is published, the main install path should be:

- release page link: `https://github.com/<owner>/<repo>/releases/latest`

Users should download one of these:

1. `wysprflow.dmg` or versioned `.dmg`
2. fallback: `wysprflow.app.zip`

Do not tell end users to use `npm` or `pnpm`. That is only for development from source.

## Install from source instead

This is the path for developers using GitHub who want to run the project without relying on a signed/notarized public build.

```bash
git clone <repo-url>
cd wysprflow
pnpm install
pnpm tauri:dev
```

Or build a local app bundle:

```bash
pnpm tauri:build:app
```

That outputs:

- `src-tauri/target/release/bundle/macos/wysprflow.app`

This source-based path does **not** require Apple Developer Program membership. Apple membership is only needed when you want a polished signed/notarized public download.

If you are testing locally as the developer, build your own DMG:

```bash
pnpm install
pnpm tauri:build:dmg
```

## Install from DMG

1. Open the DMG.
2. Drag `wysprflow.app` into `Applications`.
3. Launch `wysprflow` from `Applications`, not from the DMG window.

If you ship a `.zip` instead:

1. Download `wysprflow.app.zip`
2. Unzip it
3. Move `wysprflow.app` into `Applications`
4. Launch it

## First-run setup

You will be guided through:

1. confirming the app is running from `Applications`
2. provider keys
3. microphone permission
4. accessibility permission
5. input monitoring permission for the `Fn` hotkey

## Required keys

- `Deepgram` for speech-to-text
- `OpenRouter` or `Anthropic` for cleanup

Provider details: [providers.md](providers.md)

## First test

1. Open `TextEdit` or `Notes`
2. Click into a normal text field
3. Hold `Fn`
4. Speak a short sentence
5. Release the hotkey and wait for paste

If your Mac uses `Fn` for Emoji or Globe shortcuts, change that in `System Settings -> Keyboard`, or use the fallback shortcut shown in `wysprflow` settings.

## macOS warnings

Unsigned tester builds may trigger Gatekeeper warnings. Signed and notarized public builds are the proper launch path. See [release.md](release.md) for the signing setup.

## Recommended public install path

For the website:

- primary button -> latest GitHub Release page
- release assets -> `DMG` first, `.app.zip` as fallback

That keeps install simple and familiar for Mac users.
