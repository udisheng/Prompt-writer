# Prompt Rewriter

A small always-on-top desktop overlay. Type a rough idea, pick a generation model, and it rewrites the idea into that model's prompt format. The rewrite is done by an LLM on OpenRouter or a local Ollama server. HTTP calls are made from the Rust side.

Targets:

- **Wan 2.2** text-to-video and image-to-video, with a separate negative prompt
- **MiniMax H3**, 4–15 seconds
- **Seedance** 1.0, 2.0, and 2.5, with a duration (2.5 up to 30 seconds)
- **Seedream** generate and edit

## Run it in development

You need Node 22, Rust stable, and a WebView.

```bash
npm install
npm test          # rewriter pipeline tests, no GUI
npm run tauri dev # overlay window
```

The default hotkey is **Ctrl+Shift+Space**. Esc hides the window. Quit from the system tray. Closing the window hides it; it does not exit.

On Linux, install the Tauri system libraries first:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libgtk-3-dev
```

## Build the Windows installer

The installer is an NSIS `.exe` and a WiX `.msi`. This repository cannot cross-compile that bundle from Linux. Use a Windows machine, or the GitHub Actions workflow [`.github/workflows/windows.yml`](.github/workflows/windows.yml), which runs on `windows-latest` for pushes to `main`, version tags, and manual runs. Download the `prompt-rewriter-windows` artifact. A tagged push also creates a GitHub release.

On a Windows PC:

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload.
2. Install [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/). The NSIS installer also downloads the Evergreen bootstrapper if WebView2 is missing. Windows 11 usually already has it.
3. Install [rustup](https://rustup.rs/) (the `stable` MSVC toolchain) and Node 22.
4. In this repo:

```bash
npm install
npm test
npm run tauri build
```

The installers are written to:

- `src-tauri/target/release/bundle/nsis/` — setup `.exe` (current user, no admin prompt)
- `src-tauri/target/release/bundle/msi/` — `.msi`

The build is not code-signed. Windows SmartScreen will warn until the binary is signed.

`npm run tauri build` on Linux produces a native Linux binary instead, once the WebKit packages above are installed. That confirms the app compiles. It does not produce the Windows installer.

## Settings

Open **Settings** in the overlay, or choose Settings from the tray menu.

| Setting | Notes |
| --- | --- |
| Backend | OpenRouter or Ollama |
| OpenRouter API key | Stored in the OS keychain (Windows Credential Manager, macOS Keychain, or Linux secret service). It is not written into `settings.json` and is redacted from errors. If the keychain is unavailable, the key is stored in a private file (`openrouter.key`, mode `0600` on Unix) in the config folder. |
| OpenRouter model | Refresh loads `https://openrouter.ai/api/v1/models`. You can still type any model id. |
| Ollama URL | Default `http://localhost:11434`. Refresh reads `/api/tags`. |
| Hotkey | Default `Ctrl+Shift+Space`. Record listens for the next chord. |
| Default target | Used the next time the overlay opens. |
| Temperature | `0` to `2`, sent with the chat request. |

Both backends use one OpenAI-compatible client:

- OpenRouter: `POST https://openrouter.ai/api/v1/chat/completions`
- Ollama: `POST {base}/v1/chat/completions`

A missing key, a rejected key, a missing model, and a refused Ollama connection each surface as their own message.

## Prompt templates

Each target has a markdown file with a short frontmatter block and a system prompt. On first launch the app copies the bundled files into the config folder and reads them from there after that. Edit the copies when a model's guide changes. You do not need to recompile. **Reset to default** in Settings restores the bundled text for one file. **Open folder** opens the directory in the system file manager.

Typical locations (Settings shows the real path):

- Windows: `%APPDATA%\com.promptrewriter.app\templates`
- Linux: `~/.config/com.promptrewriter.app/templates`
- macOS: `~/Library/Application Support/com.promptrewriter.app/templates`

Frontmatter keys:

```yaml
---
id: wan22-t2v          # must match the file name
family: wan22
variant: t2v
title: Wan 2.2 · Text to video
max_chars: 800
max_chars_kind: model  # model or suggested
max_words: 100         # optional
negative_max_chars: 500
output: wan_pair       # wan_pair or text
---
```

`{{duration}}` and `{{duration_clock}}` are replaced for MiniMax H3 and Seedance. Replace `{{duration_clock}}` yourself if you add it; the app substitutes the clock form first so it is not eaten by `{{duration}}`.

What each bundled template asks the model to write:

| File | Format |
| --- | --- |
| `wan22-t2v.md` | Entity, scene, motion, aesthetic control. Positive prompt ≤ 800 characters. Negative prompt ≤ 500. JSON `{"positive","negative"}`. |
| `wan22-i2v.md` | Motion and one camera behavior only. The exact phrase `fixed camera` when the camera is still. ≤ 100 words and ≤ 800 characters. |
| `minimax-h3.md` | `integrated_multimodal_description`, `overall_soundscape`, `non_diegetic_music`, `[Shot N]`, natural-language camera moves, timeline inside 4–15 seconds, under 7000 characters. |
| `seedance-1.0.md` | Subject + action. Scene changes use `camera/scene switch`. Duration is a trailing `--duration` flag. Does not use the 2.5 "No subtitles / No BGM" closer. |
| `seedance-2.0.md` | `Shot 1`, `Shot 2`, … One camera move per shot. No timestamps. References use `@Image 1`. Constraints are subtitle-free, no logo, no watermark. |
| `seedance-2.5.md` | Asset map, one-sentence summary, integer-second shots, constants including `No subtitles` and `No BGM`. Up to 30 seconds. |
| `seedream-generate.md` | Purpose, then subject, action, environment, style and light. On-image text in double quotes. About 300 Chinese characters or 600 English words. |
| `seedream-edit.md` | Names the edit target and what stays unchanged. On-image text in double quotes. Seedream 5.0 pro/flash can take `0`–`999` coordinates. |

Wan replies are parsed as JSON, including fenced JSON and a `POSITIVE:` / `NEGATIVE:` fallback. If the split fails, the raw reply is shown in the positive box with a warning. Other models return the prompt text only. The character count turns red when a model limit or a suggested length is exceeded. The prompt is still shown and saved.

Wan's 800 / 500 and H3's 7000 are API truncation limits. Seedance's 1000 English words (or 500 Chinese characters) and Seedream's 600 English words (or 300 Chinese characters) are official quality budgets, not hard cutoffs. Change the numbers in the frontmatter if a provider publishes a different cap.

## History

The last 50 rewrites are stored in `history.json` in the config folder: idea, target, output, negative prompt, backend, model, and time. Click a row to restore it into Compose, or copy it. History stays on this machine.

## Tests

`npm test` runs the Rust crate `rewriter-core`. That covers:

- bundled templates still contain each model's format rules
- request URLs, bodies, and headers for OpenRouter and Ollama
- API keys stay out of the JSON body and out of `Debug` output
- a mock HTTP server round-trip: render a Wan template, POST a chat completion, parse positive and negative
- bad API key, missing model, Ollama not running, and delimiter-formatted Wan output
- history capped at 50
- the key file is created mode `0600` on Unix

## Caveats

- Replies arrive after the model finishes. Token streaming is not wired up.
- The overlay does not upload reference images. Image-to-video and edit templates tell the model that a picture already exists and to describe motion or the change only.
- H3 and Seedance 1.0 / 2.0 durations are clamped to 4–15 seconds. Seedance 2.5 allows 4–30 seconds.
- The global hotkey can fail if another app already owns it. Settings reports that, and the tray menu still opens the window.
- The Windows build in GitHub Actions is unsigned.
