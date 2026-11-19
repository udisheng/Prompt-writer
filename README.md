# Prompt Rewriter

Prompt Rewriter is a small always-on-top desktop overlay. You type a rough idea, pick a video or image model, and an LLM rewrites the idea into that model's prompt format.

Video and image models do not share a prompt shape. Wan 2.2 wants a short cinematic paragraph plus a negative prompt. MiniMax H3 wants labelled shots, a soundscape, and a score inside a 4–15 second timeline. Seedance 1.0, 2.0, and 2.5 each use a different shot syntax. Seedream wants a short natural-language image prompt, with edits named as a change plus what must stay. Pasting one loose sentence into all of them drops the details those models actually follow. The overlay stays above the generator you are using, so the rewrite is a copy away.

The app does not call Wan, MiniMax, Seedance, or Seedream. The rewrite itself is a chat completion sent to [OpenRouter](https://openrouter.ai/) or a local [Ollama](https://ollama.com/) server. HTTP runs in Rust. The window is a Tauri 2 shell. The published installer is the Windows one.

The prompt rules come from the research in [`docs/prompt-research.md`](docs/prompt-research.md) (17 November 2026). Each target is a markdown template under `core/templates/`.

## Features

- **Global hotkey overlay.** Default chord is **Ctrl+Shift+Space**. It shows or hides a borderless window that is always on top and skipped on the taskbar (460×760, minimum 380×560). **Esc** or **Hide** hides it. **Ctrl+Enter** rewrites.
- **System tray.** Left-click toggles the window. The menu is Show / Hide, Settings, and Quit. Quit is the way to exit. A second launch focuses the window that is already running.
- **Two rewrite backends.** OpenRouter, or Ollama on your machine. Both go through one OpenAI-compatible chat client. The request sets `"stream": false`.
- **Eight targets.**
  - Wan 2.2 text-to-video and image-to-video
  - MiniMax H3 (4–15 seconds)
  - Seedance 1.0, 2.0 (4–15 seconds), and 2.5 (4–30 seconds)
  - Seedream generate and edit
- **Wan returns two fields.** A positive prompt and a separate negative prompt, with their own limits.
- **Protected keywords.** A list you edit in Settings, in `protected-keywords.txt`, or with `/add key` in the idea box. If a listed word appears in the idea, the rewrite keeps that exact spelling. `{{double braces}}` do the same for one idea.
- **Per-model limits.** Character and word caps live in each template's frontmatter. The overlay counts the result and turns the count light red when a cap is crossed. The prompt is still shown and saved.
- **History.** The last 50 rewrites stay on this machine. Click a row to put it back into Compose, or copy it.
- **Settings.** Backend, model id, Ollama URL, hotkey, default target, and temperature (0 to 2, default 0.4).
- **OS keychain for the OpenRouter API key.** Windows Credential Manager, macOS Keychain, or the Linux secret service. The key is not written into `settings.json`. If the keychain cannot be used, the key falls back to a private file.

## Prompt formats

Compose groups the eight templates into four families. The hint under the picker is a one-line reminder. The full rules are the template body, which is sent as the system message. `{{duration}}` and `{{duration_clock}}` are filled in before that send for MiniMax H3 and Seedance. The clock form is `00:06.000` for 6 seconds, and it is replaced first so `{{duration}}` does not eat it.

The samples below are the shape examples inside the template files (and the research those lines were taken from). They show the format. They are not a claim about what a particular OpenRouter model will return for a new idea.

Limits in the table are the frontmatter the running app enforces. "API truncation" means the provider guide says text past that length is cut. "Quality budget" means the provider guide says extra length drops details. The word warning in the app still reads "word model limit" whenever `max_words` is crossed.

| Template | Output | What the app enforces | Where the number comes from |
| --- | --- | --- | --- |
| `wan22-t2v` | positive + negative | 800 characters positive, 500 characters negative | API truncation. The template aims for about 80–120 words (official outer band 60–200; a simple shot can be shorter). That band is not a separate counter. |
| `wan22-i2v` | positive + negative | 100 words and 800 characters positive, 500 characters negative | 100 words from the official I2V rewriter; 800 / 500 are API truncation. |
| `minimax-h3` | one prompt | 7,000 characters. Duration 4–15 seconds. | API character cap. |
| `seedance-1.0` | one prompt | 1,000 words. Duration 4–15 seconds. | Quality budget: 1,000 English words or 500 Chinese characters. Only the word cap is in frontmatter. |
| `seedance-2.0` | one prompt | 1,000 words. Duration 4–15 seconds. | Same Seedance quality budget. |
| `seedance-2.5` | one prompt | 1,000 words. Duration 4–30 seconds. | Same word budget. 30 seconds is the 2.5 duration ceiling. |
| `seedream-generate` | one prompt | 600 words | Quality budget: 600 English words or 300 Chinese characters. Only the word cap is in frontmatter. |
| `seedream-edit` | one prompt | 600 words | Same Seedream budget. The template tells the model to stay shorter than a full generation prompt. |

Counts are Unicode scalar values (`"字".chars().count()` is 1), which matches the Wan guide's rule that each Chinese character or Latin letter counts as one. Words are whitespace-separated. There is no separate Chinese-character counter for Seedance or Seedream.

### Wan 2.2 · Text to video

File: `core/templates/wan22-t2v.md`.

A few short sentences, in the idea's language (Chinese or English), in this order:

1. Subject first, including a count when it matters ("exactly two people") and at least two features the idea already supports.
2. Motion, with speed, direction, and the body part. Two or three beats when the idea has a sequence. A still idea gets one small plausible action. "Walks briskly" is used instead of "running fast".
3. One shot size and one camera move. A still camera is the exact phrase `fixed camera`. Leaving the camera out makes Wan add a mild push-in.
4. Scene and light. A named style goes last, and only when the idea already named one.

Wan weights the start of the prompt more than the end, so style does not lead. The useful length is about 80–120 words. The official rewriter's 60–200 word band is the outer range, and a simple shot can be shorter (about 25–80). The 800-character cap wins when those disagree. At most four aesthetic choices, counting the shot size and the camera move.

Short positive boundaries ("nobody else enters", "they stay seated") belong in the positive prompt. Few-step Wan at CFG=1 often ignores the negative field, so those constraints do not go only in the negative list.

The template is written for Wan 2.2. It leaves out the voice, sound-effect, and BGM formulas used by Wan 2.5 and later. It also carries a short Wan vocabulary (camera moves, `shallow depth of field`, `golden hour sunlight`, `side lighting`, `film grain`, and the speed words `slowly`, `gently`, `briskly`) so the rewrite uses terms Wan already follows.

The model is asked to return only:

```json
{"positive":"...","negative":"..."}
```

Shape example from the template:

```text
A young woman with wavy auburn hair and a vintage floral dress slowly lifts a ceramic cup, turns her head toward the window, and exhales a faint thread of steam. Medium close-up, center composition, the camera pushes in gently. She stands at a rain-streaked cafe window at dusk, brass lamps behind her, warm side lighting mixed with cool blue residual daylight. Live-action.
```

A negative list starts from the short official set (`low resolution, error, worst quality, low quality, deformed, extra fingers, bad proportions`) and adds anatomy, watermark, or plastic-skin terms only when this idea needs them. When the positive prompt says `fixed camera`, the negative list also excludes shaky cam and handheld. It stays at or under 500 characters.

### Wan 2.2 · Image to video

File: `core/templates/wan22-i2v.md`.

The still already defines the subject, the scene, and the style. Do not re-describe wardrobe or the room. The clip still drifts after the opening frames, so the positive prompt locks four things, in 100 words or fewer and 800 characters or fewer:

1. How many people stay in frame, when the idea names them ("exactly two people", "nobody else enters").
2. The motion, with speed and a body part, keeping the action you already named.
3. One camera behavior. A camera that should hold still includes the exact phrase `fixed camera`. A change over time is allowed in sequence (`Camera pushes in, then holds a fixed camera.`). Several moves in the same moment are out.
4. Short boundaries for what must not happen ("they remain seated"). Those stay in the positive prompt because a CFG=1 negative list often does not apply them.

Shape example from the template:

```text
Exactly two people stay in frame, and nobody else enters. The woman slowly turns her head toward the window and smiles. Steam drifts up from the cup. They remain seated. Camera pushes in, then holds a fixed camera.
```

When the positive prompt says `fixed camera`, the negative list excludes shaky cam, handheld, and extra camera moves. The same JSON pair is required. This app does not upload the still. The template tells the rewriter that the picture already exists.

### MiniMax H3

File: `core/templates/minimax-h3.md`. Variant `t2va`.

The clip length is the duration you set (default in the overlay is 6 seconds, allowed range 4–15). The prompt must stay under 7,000 characters. Three fields, labels exact, separated by a blank line:

```text
integrated_multimodal_description: [Shot 1] ...

overall_soundscape: ...

non_diegetic_music: ...
```

`[Shot 1]` opens with style and composition and has no timestamp. Later shots use `[Shot N]` and a cut time inside the duration, for example `[Shot 2] At 00:03.500, the camera cuts to ...`. Camera moves are natural English sentences (`The camera pushes in with small amplitude at slow speed toward the folded letter.`), not Hailuo 02 / 2.3 / T2V-01-Director bracket commands such as `[Pan left]`. Spoken words stay verbatim inside `<d>[Language] ...</d>`, with a stable speaker id such as `(S1)`. On-screen text stays in double quotes and is not translated.

`overall_soundscape` is ambience and physical sound, one to four sentences. `non_diegetic_music` is score the characters cannot hear, or `N/A`. There is no negative-prompt field.

If the idea describes an existing first frame, last frame, or both, the template adds a picture-alignment line (`<Picture 1>`, `Picture 2`) before the three fields. The app still does not upload those pictures.

Shape example from the template, written for a duration whose cut time fits:

```text
integrated_multimodal_description: [Shot 1] Live-action, cinematic, a medium-wide shot frames a baker opening the shutters of a small street bakery before sunrise. The camera pushes in with small amplitude at slow speed as the middle-aged baker with a calm, slightly raspy voice (S1) places a fresh loaf on the wooden counter and says: <d>[English] First batch of the morning.</d> [Shot 2] At 00:05.000, the camera cuts to a close-up of steam rising from the sliced bread while the baker's final words carry over from the previous shot.

overall_soundscape: Wooden shutters scrape open, followed by the soft thud of a loaf on the counter and a faint hiss of steam. Distant early-morning traffic stays in the background.

non_diegetic_music: A sparse acoustic guitar figure at a slow tempo, fading under the close-up.
```

### Seedance 1.0

File: `core/templates/seedance-1.0.md`.

Subject plus action, then the scene, one camera move, and a style only when the idea needs them. Several actions stay in time order (`then`). A scene change uses the phrase `camera/scene switch`, or `Switch to a medium shot:`, or `The camera switches to...`. This version does not use `Shot 1` numbering or timestamps, and it does not add Seedance 2.5's `No subtitles` / `No BGM` closer.

Flags sit after the prose. The prompt ends with `--duration` and the chosen length. `--camerafixed true` is added only when the camera must stay locked. A named resolution can be `--resolution 1080p`.

Shape example from the template (the file uses `5` as a sample; the system prompt substitutes the duration you picked):

```text
The ship cuts through the storm as lightning repeatedly splits the night sky. Switch to a medium shot: a captain stands on the deck, holding a vintage telescope and gazing into the distance. The camera slowly pushes forward as he stows the telescope, his expression resolute as he looks ahead.
--duration 5
```

### Seedance 2.0

File: `core/templates/seedance-2.0.md`.

Engineering-style shots, not timestamps. Seedance 2.0 does not reliably follow ranges such as `0-3 seconds`. The template uses `Shot 1`, `Shot 2`, `Shot 3`, and lets the model pace them across the chosen duration (4–15 seconds).

Each shot is one camera move or transition, then action and expression, then spatial change, then audio. References bind by upload order: `@Image 1`, `@Video 1`, `@Audio 1`. The closer uses the wordings the template lists: `keep it subtitle-free`, `avoid generating any text or subtitles`, `do not generate a logo`, `do not generate a watermark`. There is no separate negative field.

Shape example from the template:

```text
Shot 1: Side shot of a street alley; the man slowly starts running, with a sense of rapid breathing.
Shot 2: The man knocks over a fruit stand; the camera shakes quickly and gives a close-up of the man's frightened face.
Shot 3: The man climbs over a low wall and disappears; the camera slowly pulls back and freezes on the empty street.
```

The template points out that Shot 2 in that official sample stacks a shake and a close-up, and tells the rewriter to keep a single camera move per shot.

### Seedance 2.5

File: `core/templates/seedance-2.5.md`.

Timestamps are in this version. The template asks for four blocks:

```text
Asset map:
Image 1: ...
No reference assets.

Summary:
<one sentence: subject, location, event, style, overall camera>

Shots:
Shot 1 (0-3s): <visuals, one camera move, action, dialogue, sound>
Shot 2 (3-8s): ...

Constants:
No subtitles.
No BGM; generate only environmental sounds and action sounds.
```

Ranges use 1-second steps, stay continuous, and the last range ends at the chosen duration (4–30 seconds). Other accepted time forms include `0-3 seconds` and `At the 5-second mark`. Each shot still gets one camera move. An edit uses words such as add, remove, replace, or change, and says what stays. There is no separate negative field. Sound control is the `No BGM` line, or `No audio.` when the idea wants silence.

Shape example included in the template (fit the ranges to the duration you set; the instructions also ask for the Asset map, Summary, Shots, and Constants headings):

```text
Realistic nature documentary style, natural lighting and shadows. On a warm afternoon, on a grassy slope in the forest, a chubby panda cub rolls down the hill.

0s-3s: A panda cub lies on a green grassy slope and begins to slowly roll sideways down the slope. Sunlight filters through the trees from the upper left.
3s-8s: The panda rolls toward the lower right, stops, shifts onto its belly, turns its face toward the camera, and makes a soft humming sound.

Natural environmental audio only, including wind, rustling grass, and the soft plop of the panda rolling. No subtitles. No BGM.
```

### Seedream · Generate

File: `core/templates/seedream-generate.md`.

For Seedream 4.0, 4.5, or 5.x: short, precise natural language, not a comma-separated tag stack, in about 80 to 200 words. The prompt comes back as labelled lines so each part is easy to find and edit. Each line holds one or two full sentences, and a line with nothing to say is left out:

`Purpose:` · `Action:` · `Subjects:` · `Outfits:` · `Background:` · `Lighting:` · `Camera:` · `Text:` · `Style:`

That keeps the official order: a use first when there is one (poster, logo, product photo, infographic, UI, storyboard), then subject and action, environment, and style or light. Every string that must appear in the image is in double quotes, verbatim, with its position. A series or an exact count is how 4.0, 4.5, and 5.0 lite ask for several outputs. The longer 3.0 form is used only when the idea says 3.0.

The template also carries field notes from drawing story scenes with Seedream (see [`docs/prompt-research.md`](docs/prompt-research.md#8-field-notes-seedream-in-practice)):

- **One pose and one main action per person**, with both hands placed or out of frame. Stacked actions merge bodies and add limbs.
- **Reference images bind by number** ("the woman from Image 1"). Their faces, hair and bodies are not re-described, and a portrait supplies the face only: the scene's outfit is written in words.
- **First-person point of view** keeps the viewer out of the picture and never names them.
- **Era and costume** are stated for everyone ("no modern clothes" for a historical setting).
- Unless the idea asks for writing, the Style line ends with "No text, captions or watermarks."

Shape example from the template:

```text
Purpose: A product poster for a ceramic coffee company.
Action: A handmade red ceramic mug with a slightly uneven glaze sits on a weathered oak table, steam curling from it.
Background: A quiet kitchen counter with a window on the left and an empty wall behind.
Lighting: Warm morning light from the window on the left, soft shadows to the right.
Camera: Close three-quarter view at table height, shallow depth of field.
Text: The poster title "First Batch" sits at the top, and a small caption "Roasted at dawn" sits at the bottom.
Style: Food photography, clean editorial layout with space at the top for the title. No extra logos.
```

### Seedream · Edit

File: `core/templates/seedream-edit.md`.

One or two sentences. Start with add, remove, replace, or modify. Name the target by appearance, position, or role. State the end result and what stays unchanged (pose, identity, background, typography, layout). New words that must appear go in double quotes. An edit that touches a person keeps their face and identity unchanged and changes one thing about them per sentence; a new outfit, pose and place in one edit tend to change the face too.

On Seedream 5.0 pro or flash, a point is `x y` and a box is `x1 y1 x2 y2`, normalized from 0 to 999, with `0 0` at the top left. The template's example of that form is `Replace the person on the left in Image 1 120 180 640 760 with a robot.`

Shape example from the template:

```text
Dress the tallest panda in pink Peking Opera costume and headgear, keeping its pose unchanged.
```

The source image is not attached. The prompt has to name the change clearly enough to apply to whatever image you already have in the Seedream UI.

## Protected keywords

A protected keyword is copied into the rewrite exactly: same spelling, case, and punctuation. The rewriter does not translate it, paraphrase it, or drop it. This applies to every target, not only Wan. LoRA trigger words, character names, and style tags are the usual list.

Three ways to set them, all stored in the same file:

| Way | What you do |
| --- | --- |
| Settings | **Protected keywords**, one word or phrase per line, then **Save keywords**. |
| The file | `protected-keywords.txt` in the config folder (next to `settings.json`). One keyword per line. A line that starts with `#` is a note. A JSON array, or `{"keywords": ["Mika"]}`, is read if you put that in the file by hand. Saving from the app writes lines. |
| The idea box | A line that starts with `/` is a command. It is not sent to the model. The reply appears in the output box. |

Commands, with names that ignore case:

| Command | Effect |
| --- | --- |
| `/show key` | Numbered list of the saved keywords. |
| `/add key Mika, neon noir, zxc_lora` | Adds each comma-separated phrase. The text is stored as typed. A duplicate is skipped. |
| `/remove key neon noir` | Removes that phrase. A number removes that row from `/show key`, unless a keyword is exactly that number. |
| `/clear key` | Does not delete anything. |
| `/clear key confirm` | Deletes the whole list. |
| `/help` | Prints the command list. |

Typing `/` shows the matching commands under the idea box. Click one to fill it in.

For a single idea, wrap a term in double braces: `a portrait of {{Mika}} in {{neon noir}}`. Those terms are protected for that request even when they are not in the file, and the braces are removed from the rewrite.

On a normal rewrite the app does both of these:

1. It adds the matching terms to the system prompt, so the model is told to copy them.
2. After the reply, Rust checks that each of those terms is still in the prompt. If any are missing, it sends one corrective follow-up. If they are still missing, the output shows a warning that names them, and **Append missing terms** adds them to the end of the prompt.

A keyword matches wherever that exact string occurs in the idea, including inside a longer word, so short generic words are a poor fit for the list.

## Architecture

```text
overlay (src/, Vite + TypeScript)
    invoke()  →  Tauri commands (src-tauri/)
                    files, keychain, hotkey, tray
                         ↓
                  rewriter-core (core/)
                    templates, settings, history, HTTP
                         ↓
              OpenRouter or Ollama chat completions
```

The window asks Rust to call the provider. The saved key stays in the Rust process. Settings receives only whether a key is configured, and the password field is cleared after **Save key**. `src/api.ts` is a thin `invoke` wrapper. Outside the desktop shell (plain `npm run dev`) it shows a layout preview with fake model lists and does not send a request. A note at the top of that preview says so.

`src-tauri` owns the process:

- `src/lib.rs` starts the window, seeds files, registers the hotkey, installs the tray, and turns a close request into hide.
- `src/commands.rs` is the command surface: settings, key, model lists, templates, rewrite, history.
- `src/secrets.rs` talks to the OS keychain (`keyring`, service `com.promptrewriter.app`, account `openrouter`).
- `src/files.rs` reads and writes the config folder.
- `src/window.rs` parses shortcuts, toggles the window, and builds the tray menu.

`rewriter-core` is a plain library with no GUI. On rewrite it:

1. Trims the idea and rejects an empty idea or one longer than 8,000 characters.
2. Loads `{id}.md` from the config templates folder (and writes the bundled copy if that file is missing).
3. Checks duration: MiniMax H3 and Seedance 1.0 / 2.0 are 4–15 seconds, Seedance 2.5 is 4–30, and Wan and Seedream ignore duration.
4. Renders the system prompt and a user message that names the target, the duration when there is one, and the rough idea. Protected keywords that appear in the idea, and any `{{double brace}}` terms, are appended to the system prompt.
5. Builds a chat request and posts it. Timeout is 10 seconds to connect and 120 seconds overall. The user agent is `PromptRewriter/0.1`.
6. Parses the reply, strips braces around inline protected terms, and checks that every required term is still present. If any are missing, it retries once with a corrective message. Terms that are still missing are returned as a warning.
7. Measures the result against the template caps, prepends a history entry, and returns the text.

A line that starts with `/` never reaches this path. It is handled as a keyword command and written to `protected-keywords.txt` immediately.

Wan replies are parsed as JSON (`positive` / `positive_prompt` / `prompt`, and `negative` / `negative_prompt`), including a fenced block or a JSON object wrapped in prose. A `POSITIVE:` / `NEGATIVE:` or `---POSITIVE---` / `---NEGATIVE---` split is the fallback. If that fails, the raw reply is placed in the positive box with the warning "Could not split a positive and negative prompt. The raw reply is in the positive box." Other targets return the prompt text, with a markdown fence removed if the model added one.

### Where files live

Tauri's app config directory for identifier `com.promptrewriter.app`. Settings shows the path this machine is actually using.

| OS | Typical config folder |
| --- | --- |
| Windows | `%APPDATA%\com.promptrewriter.app` |
| Linux | `~/.config/com.promptrewriter.app` |
| macOS | `~/Library/Application Support/com.promptrewriter.app` |

| File | Role |
| --- | --- |
| `settings.json` | Backend, model ids, Ollama URL, hotkey, default template, temperature. No API key. |
| `history.json` | `{ "entries": [ ... ] }`, newest first, capped at 50. |
| `openrouter.key` | Fallback only, when the OS keychain cannot store the key. Mode `0600` on Unix. |
| `templates/*.md` | The templates the running app reads. |
| `protected-keywords.txt` | Protected keywords, one word or phrase per line. |

A `settings.json` or `history.json` that does not parse is renamed to the same name plus `.bak`, and the app continues with defaults or an empty history.

### Editing templates

On first launch, each bundled file is copied into `templates/` if that name is not already there. Later launches leave your edits in place. An updated install does not overwrite a template you have already customized.

Change a live template in either place:

- **Settings → Prompt templates.** Pick the file, edit, **Save template**. The next rewrite uses it. The frontmatter `id` has to match the file. A template larger than 100,000 characters is rejected.
- **Open folder**, then edit the markdown in any editor.

**Reset to default** writes that one file back from the copy compiled into the binary (`include_dir` on `core/templates/`). Changing the bundled default means editing `core/templates/*.md` and rebuilding. Reset after that rebuild picks up the new bundled text. Until you reset, the config-folder copy remains the one that runs.

Frontmatter the parser accepts:

```yaml
---
id: wan22-t2v          # must match the file name; letters, digits, . _ -
family: wan22
variant: t2v
title: Wan 2.2 · Text to video
max_chars: 800         # optional
max_chars_kind: model  # model (default) or suggested
max_words: 100         # optional
negative_max_chars: 500
output: wan_pair       # wan_pair or text
---
```

`max_chars_kind: model` labels the counter "limit" and the warning "model limit". `suggested` labels them "suggested" and "suggested length". None of the bundled files set `suggested`. Seedance and Seedream express their budget with `max_words` only. Crossing `max_chars` or `max_words` sets `overLimit`, adds a warning, and turns the count light red (`#ff9b9b`). Crossing 90% of `max_chars` turns it amber. The text is still kept.

`{{duration}}` becomes the number of seconds. `{{duration_clock}}` becomes `00:SS.000` (seconds are capped at 59 in the clock string). Put the clock placeholder in the template yourself if you add a new file that needs it.

## Requirements (Windows)

The installer targets Windows. To build it locally you need:

1. **Node.js 22.** That is the version [`.github/workflows/windows.yml`](.github/workflows/windows.yml) uses. This repo's frontend build was run on Node 22.
2. **Rust stable via [rustup](https://rustup.rs/).** `rust-toolchain.toml` selects the `stable` channel. The crates use Rust edition 2024, so the toolchain has to be recent enough for that edition. Install the MSVC target that rustup selects by default on Windows (`stable-x86_64-pc-windows-msvc`).
3. **Microsoft C++ Build Tools.** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload. Tauri links the native shell with the MSVC toolchain.
4. **WebView2.** Windows 11 usually already has the Evergreen runtime. The NSIS bundle is configured with `webviewInstallMode: downloadBootstrapper` (silent), so the installer downloads the bootstrapper when the runtime is missing. You still want the runtime installed for `tauri dev`.

The NSIS package installs for the current user (`installMode: currentUser`), so the setup executable does not ask for an administrator prompt.

On Debian or Ubuntu, compiling the same shell for a local run needs the Tauri 2 WebKitGTK libraries:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libgtk-3-dev
```

That Linux build is a native binary. It does not produce the Windows installer.

## Develop

```bash
npm install
npm test            # rewriter-core tests, no window
npm run tauri dev   # overlay on http://127.0.0.1:1420
```

`npm run dev` alone is the Vite layout preview on port 1420. Model calls, the tray, and the global hotkey run under `npm run tauri dev`.

The default hotkey is **Ctrl+Shift+Space**. If another app already owns it, the window still opens from the tray, and Settings shows the registration error.

## Tests

`npm test` runs `cargo test --manifest-path core/Cargo.toml`. The 28 tests in `rewriter-core` cover:

- bundled templates still contain each model's format rules
- request URLs, bodies, and headers for OpenRouter and Ollama
- API keys stay out of the JSON body and out of `Debug` output
- a mock HTTP round-trip: render a Wan template, POST a chat completion, parse positive and negative
- a rejected API key, a missing model, Ollama not running, and a delimiter-formatted Wan reply
- history capped at 50
- the key file created mode `0600` on Unix
- duration bounds, frontmatter parsing, and secret redaction
- protected-keyword file parsing, `{{double brace}}` extraction, the post-check that a term survived the rewrite, and the `/add key` command parser

The Tauri crate has one extra test that only checks hotkey string parsing. `npm test` does not build the GUI crate.

## Build the Windows installer

### On a Windows machine

```bash
npm install
npm test
npm run tauri build
```

`tauri.conf.json` runs `npm run build` first (`tsc` and Vite). Installers land in:

- `src-tauri/target/release/bundle/nsis/` — setup `.exe` (current user)
- `src-tauri/target/release/bundle/msi/` — `.msi`

The build is not code-signed. Windows SmartScreen warns on an unsigned setup executable until the binary is signed.

### GitHub Actions

[`.github/workflows/windows.yml`](.github/workflows/windows.yml) is named **Windows installer**. It runs on a push to `main`, on a tag that starts with `v`, and from **Run workflow**.

1. **Core tests** on `ubuntu-latest`: `cargo test --manifest-path core/Cargo.toml`.
2. **MSI and NSIS** on `windows-latest`, after tests pass. It installs Node 22, installs npm dependencies, and runs `tauri-apps/tauri-action@v0` with `--bundles nsis,msi`.
3. The installers are uploaded as the artifact **`prompt-rewriter-windows`**. The job fails if those files are missing. Download it from the workflow run.

The workflow does not configure a signing certificate. Those artifacts are unsigned, and SmartScreen applies the same warning as a local build.

A push of a `v*` tag also lets `tauri-action` create a GitHub Release for that tag (`permissions: contents: write` is set for this). The Actions artifact is still uploaded on that run.

## Configuration

Open **Settings** in the overlay, or choose Settings from the tray. **Save settings** writes `settings.json` and re-registers the hotkey. A hotkey that fails to register leaves the previous chord in place.

| Setting | Default | Notes |
| --- | --- | --- |
| Backend | OpenRouter | `openrouter` or `ollama` |
| OpenRouter API key | empty | Keychain, or `openrouter.key` if the keychain is unavailable. One token, no spaces, 8–512 characters. The field is cleared after a successful save. |
| OpenRouter model | empty | You have to choose one. **Refresh** loads `https://openrouter.ai/api/v1/models`. Free models (prompt price 0) are marked and listed first. You can still type any model id. The placeholder text in the box is only an example of the id shape. |
| Ollama URL | `http://localhost:11434` | Must be `http` or `https`. **Refresh** reads `{origin}/api/tags`. |
| Ollama model | empty | Type a name or pick one from Refresh. The placeholder `llama3.2` is not a default. |
| Hotkey | `Ctrl+Shift+Space` | **Record** listens for the next chord. Hold Ctrl, Alt, Shift, or the Windows key (saved as `Super`), then the main key. Esc cancels recording and leaves the window open. |
| Default target | Wan 2.2 text to video | Used the next time the overlay opens. |
| Temperature | 0.4 | Slider from 0 to 2 in steps of 0.1. Values are clamped to 0–2. |

Saved settings look like this:

```json
{
  "backend": "openrouter",
  "openrouterModel": "example/model",
  "ollamaBaseUrl": "http://localhost:11434",
  "ollamaModel": "",
  "hotkey": "Ctrl+Shift+Space",
  "defaultTemplateId": "wan22-t2v",
  "temperature": 0.4
}
```

### OpenRouter

The chat call is `POST https://openrouter.ai/api/v1/chat/completions` with `Authorization: Bearer <key>`, `HTTP-Referer: https://prompt-rewriter.local`, and `X-Title: Prompt Rewriter`. The body is the model id, the clamped temperature, `"stream": false`, a system message (the rendered template), and a user message (target title, duration when the template uses one, and the idea). The key is not copied into the JSON body. Provider errors are redacted before they are shown: bearer tokens, `sk-` keys, and the saved secret become `[redacted]`.

If listing models returns 401 or 403 for the saved key, the list is retried once without the key.

### Ollama

The chat call is `POST {origin}/v1/chat/completions`. A base URL that already ends in `/v1` keeps that prefix. No `Authorization` header is sent, even if an OpenRouter key is stored. Pull models with the Ollama CLI (`ollama pull <name>`), then Refresh.

Start the server with `ollama serve` when the app reports that Ollama is not running. The URL in that message is the base URL from Settings.

## Troubleshooting

| What you see | What it means |
| --- | --- |
| Add an OpenRouter API key in Settings. | Backend is OpenRouter and no key is stored. |
| Paste the API key as a single token, with no spaces. | The key was empty, contained whitespace, or was shorter than 8 or longer than 512 characters. |
| OpenRouter rejected the API key. | The provider returned 401 or 403. |
| Choose a model in Settings. | The model box for the selected backend is empty. |
| OpenRouter does not have model `…`. | The id was rejected as unknown. Pick another id. |
| Could not reach OpenRouter. | The connection failed. |
| Ollama is not running at `…`. | Nothing accepted the connection. Start `ollama serve`, or change the base URL. |
| Ollama does not have model `…`. | Pull it (`ollama pull <name>`) and refresh the list. |
| The request timed out. | No response within 120 seconds. The model may still be loading. |
| The provider rate-limited the request. | HTTP 429. |
| Could not register `Ctrl+Shift+Space`. Another app may already own it. | The global shortcut is taken. Settings keeps the error. The tray still opens the window. Pick another chord and save. |
| Write an idea before rewriting. / The idea is longer than 8,000 characters. | The idea box is empty, or past the cap (`maxlength` is 8000). |
| Duration has to be a whole number from 4 to 15 seconds. | The overlay checks the duration box before the request. Seedance 2.5 uses 4 to 30. The Rust command reports `MiniMax H3 duration must be between 4 and 15 seconds (got N).` The duration box is hidden for Wan and Seedream. |
| Could not split a positive and negative prompt. | The Wan reply was not JSON and had no POSITIVE / NEGATIVE markers. The raw text is in the positive box. |
| `wan22-t2v.md: frontmatter …` | That template file failed to parse. The banner lists invalid files. Fix it or use Reset to default. |
| The prompt is N characters, over the 800 character model limit. | Shown under the result. The count is light red. The text is still there and still saved. A word warning uses the same pattern ("over the 1000 word model limit"). |
| Protected terms missing from the rewrite: … | The model dropped a keyword or a `{{term}}` after one retry. **Append missing terms** adds them to the end of the prompt. |
| Unknown command. Type /help… | The idea starts with `/`, so it was handled as a keyword command and was not rewritten. |

If Settings suddenly returns to defaults, look for `settings.json.bak` in the config folder. The previous file did not parse.

## Known limitations

- Replies arrive after the model finishes. `"stream": false` is hard-coded, and the UI waits on the whole completion.
- The overlay does not upload reference images, first frames, or audio. Image-to-video and edit templates tell the model that the picture already exists and to describe motion or the change only. `@Image 1` in a Seedance prompt is an instruction for the generator UI, not an upload from this app.
- Closing the window, Esc, and Hide all hide to the tray. The process keeps running until **Quit** on the tray menu.
- The global hotkey can fail when another app owns the chord. A press is also ignored if it repeats within 350 ms.
- H3 and Seedance 1.0 / 2.0 durations are whole seconds from 4 to 15. Seedance 2.5 allows 4 to 30. Wan and Seedream have no duration control.
- History is the last 50 rewrites on this machine, in `history.json`. There is no account sync.
- A result that crosses a cap is still shown, copied, and stored. The app does not truncate it to the limit.
- The Windows installers from a local build and from GitHub Actions are unsigned.
- This repository does not cross-compile the Windows installer from Linux. Use a Windows machine or the Actions workflow.
- The browser preview (`npm run dev` outside Tauri) does not store a real API key and does not call a model. Its template list is a stand-in for layout. Limits you see there are not the bundled frontmatter. Keyword commands in that preview are stored in the browser only.
- An idea that starts with `/` is a keyword command. It is not rewritten.

## Project structure

```text
.
├── .github/workflows/windows.yml   # core tests, then the Windows MSI and NSIS artifact
├── core/
│   ├── Cargo.toml                  # rewriter-core library
│   ├── src/
│   │   ├── error.rs                # missing key, missing model, HTTP failures
│   │   ├── history.rs              # last-50 list
│   │   ├── http.rs                 # OpenRouter and Ollama client
│   │   ├── parse.rs                # Wan JSON / markers, length checks
│   │   ├── protect.rs              # protected keywords, braces, slash commands
│   │   ├── redact.rs               # strip secrets from errors
│   │   ├── secret_file.rs          # private-file fallback, mode 0600
│   │   ├── settings.rs             # defaults and temperature clamp
│   │   └── template.rs             # frontmatter, duration, bundled files
│   └── templates/                  # eight markdown templates compiled into the binary
├── docs/prompt-research.md         # source research for those templates
├── index.html                      # overlay layout
├── package.json                    # npm test, npm run build, tauri scripts
├── rust-toolchain.toml             # stable
├── src/
│   ├── api.ts                      # invoke wrappers and the browser preview
│   ├── main.ts                     # Compose, History, Settings
│   └── styles.css
├── src-tauri/
│   ├── capabilities/default.json   # which commands the window may call
│   ├── icons/                      # window and installer icons
│   ├── permissions/commands.toml
│   ├── src/
│   │   ├── commands.rs             # rewrite and settings commands
│   │   ├── files.rs                # config folder, template seed
│   │   ├── lib.rs                  # Tauri setup, tray, single instance
│   │   ├── secrets.rs              # OS keychain
│   │   └── window.rs               # hotkey, hide-on-close, tray menu
│   ├── Cargo.toml
│   └── tauri.conf.json             # product name, window, NSIS/WebView2
├── tsconfig.json
└── vite.config.ts                  # dev server on 127.0.0.1:1420
```

`npm run build` writes `dist/`, which Tauri packages. `core/target/` and `src-tauri/target/` are build output. None of those directories are part of the source.
