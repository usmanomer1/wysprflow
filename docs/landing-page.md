# Landing Page Kit

Use this file as the source of truth for the first public landing page.

## Primary CTA

Start with the release page, not a hardcoded asset URL:

- `https://github.com/<owner>/<repo>/releases/latest`

Once you have a stable asset filename, you can use:

- `https://github.com/<owner>/<repo>/releases/latest/download/<asset-name>.dmg`

GitHub documents the current pattern here:

- [Linking to releases](https://docs.github.com/repositories/releasing-projects-on-github/linking-to-releases)

## Suggested hero copy

Headline:

`Voice dictation for macOS that stays out of your way.`

Subhead:

`Hold a hotkey, speak naturally, and paste cleaned-up text into any app using your own provider keys.`

## Suggested feature bullets

- `Fast hold-to-talk dictation`
- `Bring-your-own Deepgram and cleanup provider keys`
- `Dictionary, snippets, and run history`
- `No project-owned transcription backend`

## Suggested CTA labels

- `Download for macOS`
- `View on GitHub`
- `See setup guide`

## What users should download

For non-technical users:

1. `DMG` is the main install file
2. `.app.zip` is the fallback if you also upload it

Do not use `npm` / `pnpm` as a user-facing install method. That is only for developers cloning the source code.

## Suggested secondary links

- GitHub repository
- latest release
- setup guide
- provider guide
- troubleshooting guide

## Recommended launch path

1. landing page button goes to `releases/latest`
2. GitHub draft release is reviewed and published
3. users download the `DMG`
4. once asset naming is stable, move CTA to the direct `.dmg` asset
