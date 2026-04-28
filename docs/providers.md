# Providers

`wysprflow` is bring-your-own-key. The app talks directly to the providers you configure.

## Current matrix

| Provider | Purpose | Current status | Notes |
| --- | --- | --- | --- |
| Deepgram | speech-to-text | live | default and required today |
| OpenRouter | cleanup | live | default routed cleanup provider |
| Anthropic | cleanup | live | direct alternative to OpenRouter |
| Groq | future STT | storage only | key can be saved but pipeline does not use it yet |
| OpenAI | future STT / cleanup | storage only | key can be saved but pipeline does not use it yet |
| ElevenLabs | future STT | storage only | key can be saved but pipeline does not use it yet |

## Required today

You need:

- one `Deepgram` key
- one cleanup key from `OpenRouter` or `Anthropic`

## Recommended setup

- `Deepgram` for transcription
- `OpenRouter` for cleanup when you want routing flexibility
- `Anthropic` direct when you want the shortest provider chain

## Key sources

- Deepgram: [console.deepgram.com/project/keys](https://console.deepgram.com/project/keys)
- OpenRouter: [openrouter.ai/settings/keys](https://openrouter.ai/settings/keys)
- Anthropic: [console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys)
- Groq: [console.groq.com/keys](https://console.groq.com/keys)
- OpenAI: [platform.openai.com/api-keys](https://platform.openai.com/api-keys)
- ElevenLabs: [elevenlabs.io/app/settings/api-keys](https://elevenlabs.io/app/settings/api-keys)

## Privacy note

The app does not proxy requests through a project-owned backend. Your machine connects straight to the provider APIs.
