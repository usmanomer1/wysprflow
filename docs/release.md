# Release Guide

This repo is set up to ship macOS releases through GitHub Releases.

Apple Developer membership is only needed for signed/notarized public distribution. It is **not** required for developers to clone the repo, run `pnpm tauri:dev`, or build an unsigned local `.app`.

## What exists in the repo now

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `pnpm tauri:build:dmg`

## Local release artifact

Build a local DMG:

```bash
pnpm install
pnpm tauri:build:dmg
```

Or build the app bundle only:

```bash
pnpm tauri:build:app
```

Artifacts land under:

- `src-tauri/target/release/bundle/dmg/`
- `src-tauri/target/release/bundle/macos/`

If DMG creation fails in a restricted or headless environment, the `.app` bundle is still the useful local artifact. The GitHub release workflow is the better path for the public downloadable DMG.

You can also zip the app bundle for a fallback website download:

```bash
ditto -c -k --sequesterRsrc --keepParent src-tauri/target/release/bundle/macos/wysprflow.app src-tauri/target/release/bundle/macos/wysprflow.app.zip
```

## Version bump checklist

Before tagging a release, update:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `CHANGELOG.md`

Keep the version the same in all app metadata files.

## GitHub Actions setup

In the GitHub repository:

1. enable Actions
2. set workflow permissions to `Read and write`
3. push the repo

The release workflow triggers on tags that match `v*`.

## Tagging a release

Example for version `0.1.0`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That will create or update a draft GitHub Release and upload the built macOS bundles.

## Signing and notarization

Unsigned builds are fine for private testers. Public macOS distribution should be signed and notarized.

If you want signed GitHub builds, add these repository secrets:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`

For notarization, add either the Apple ID path:

- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

Or the App Store Connect API path:

- `APPLE_API_KEY`
- `APPLE_API_ISSUER`
- `APPLE_API_KEY_P8`

The release workflow writes `APPLE_API_KEY_P8` to a temporary `.p8` file on the runner and sets `APPLE_API_KEY_PATH` for Tauri automatically.

Tauri’s current docs cover these variables and the macOS signing flow:

- [GitHub pipeline guide](https://v2.tauri.app/distribute/pipelines/github/)
- [DMG guide](https://v2.tauri.app/distribute/dmg/)
- [macOS signing guide](https://tauri.app/distribute/sign/macos/)
- [Tauri environment variables](https://v2.tauri.app/reference/environment-variables/)

## First public download link

Once the first release is live, the safest landing-page button is:

- `https://github.com/<owner>/<repo>/releases/latest`

GitHub’s current release-link docs are here:

- [Linking to releases](https://docs.github.com/repositories/releasing-projects-on-github/linking-to-releases)

After you know the exact asset filename, you can switch to a direct download URL.

## What end users install

End users should install:

1. `DMG` first
2. `.app.zip` as a fallback

End users should not install via `npm`, `pnpm`, or `cargo`. Those are source/developer flows only.
