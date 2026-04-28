# Troubleshooting

## The app opens to a blank white window

- Relaunch with `pnpm tauri:dev`
- The app now includes a startup error boundary; read the error text instead of guessing
- Validate with:

```bash
pnpm build
pnpm check:rust
```

## Dictation hears me but nothing pastes

- Confirm `Accessibility` is granted
- Make sure the target app has a normal focused text field
- Check the in-app `Run Log`
- If you are using the `Fn` hotkey, try the fallback hotkey from Settings

## The `Fn` key opens Emoji or system shortcuts

macOS can intercept `Fn`.

Change:

- `System Settings -> Keyboard -> Press Fn key to -> Do Nothing`

Or use a different hotkey in Settings.

## API key validation fails

- Verify the key really belongs to the selected provider
- Check that the provider account is active
- Try validation again from Settings
- For `Groq`, `OpenAI`, and `ElevenLabs`, note that key storage exists before pipeline support does

## `tauri dev` keeps asking for Keychain access

Debug builds are configured to use a local dev secrets file, not Keychain. If you still see prompts, make sure you are running the current code and not an older build artifact.

## The packaged app says it is damaged or unverified

That is a distribution problem, not a dictation bug.

- unsigned builds are acceptable for private testing only
- public builds should be signed and notarized

See [release.md](release.md).
