# Prompt-writing research for Wan 2.2, MiniMax H3, Seedance, and Seedream

**Research date:** 17 November 2026 (Asia/Singapore, UTC+8)  
**Scope:** Official docs and official model guides first; then fal.ai, Replicate, ComfyUI, and well-regarded community guides. This document is planning research only. No application code is included.

The planned Windows overlay app rewrites a user's rough prompt into the **target model's preferred full prompt text**. The rewriter itself talks to an LLM via OpenRouter or local Ollama (see [Section 5](#5-app-note-openrouter-and-ollama-apis)). It does not call the video/image generators.

---

## Snapshot of current versions (as of 17 Nov 2026)

| Family | Versions this guidance covers | Notes |
| --- | --- | --- |
| **Wan (Alibaba)** | **Wan 2.2** is the requested target. Official prompt formulas are shared across the Wan family. Sound/audio formulas apply only to **Wan 2.5 / 2.6 / 2.7 / 3.0**, not 2.2. | Open-weights 2.2 variants: `Wan2.2-T2V-A14B`, `Wan2.2-I2V-A14B`, `Wan2.2-TI2V-5B` (Apache 2.0). Cloud Model Studio also lists `wan2.2-t2v-plus` and I2V plus/flash. Later Wan versions exist; do not apply their audio formulas to 2.2. |
| **MiniMax** | User wrote **"MiniMax H3"**. This is almost certainly the official **MiniMax-H3** video model (Hailuo 03 / Hailuo 3.0 generation), **not** Hailuo 02 / 2.3. See [Section 2](#2-minimax-h3). | Official IDs: `MiniMax-H3`, `MiniMax-H3-Highspeed`, `MiniMax-H3-Lite`, `MiniMax-H3-Max`. Older Hailuo models still exist and use a different camera syntax. |
| **Seedance (ByteDance video)** | **1.0 Pro / Pro Fast**, **2.0 series**, **2.5 / 2.5 flash**. A **1.5 Pro** official guide also exists. Prompt syntax is **not identical** across generations. | 2.5 is the current flagship formula (timestamps + Shot N, up to 30s, multimodal refs). 2.0 does **not** reliably follow timestamps. 1.0 uses `camera/scene switch` and trailing `--parameter` flags. |
| **Seedream (ByteDance image)** | **3.0** (legacy), official **4.0 / 4.5** prompt guide, current **5.0 lite / 5.0 pro / 5.0 flash**, and **5.5** already listed in the BytePlus image API. | 4.0+ prefers concise natural language over ornate keyword stacks. Quoted on-image text is official through 4.x and still recommended in 5.x community/platform guides. 5.0 pro/flash add coordinate-based interactive editing. |

---

## 1. Wan 2.2 (Alibaba, T2V / I2V)

### Versions this section applies to

- **Primary:** Wan 2.2 text-to-video and image-to-video (`Wan2.2-T2V-A14B`, `Wan2.2-I2V-A14B`, hybrid `Wan2.2-TI2V-5B`; cloud `wan2.2-*-plus` / flash).
- Official Alibaba Cloud Model Studio prompt guide (last updated **11 Oct 2026**) is the family guide for "Wan video generation." The **formulas below are official for 2.2**. The **Sound formula is explicitly labeled Wan 3.0/2.7/2.6/2.5 only** and should not be assumed for 2.2.
- Official open-weights rewriter system prompts live in the Wan2.2 GitHub repo (`wan/utils/system_prompt.py`) and are written specifically for the **A14B T2V and I2V** models.

### Recommended structure / formula templates

**T2V basic (official):**

```text
[Entity] + [Scene] + [Motion]
```

- **Entity:** the main subject (person, animal, plant, object, or imaginary being).
- **Scene:** environment, background and foreground, real or imaginary.
- **Motion:** what moves, from stillness to slight, large, local, or overall movement.

**T2V advanced (official; preferred for finished shots):**

```text
[Entity description] + [Scene description] + [Motion description] + [Aesthetic control] + [Stylization]
```

- **Entity / scene descriptions:** adjectives and short phrases, not vague abstractions.
- **Motion description:** amplitude, speed, and effect (e.g. "swaying violently," "moving slowly," "shattering the glass").
- **Aesthetic control:** light source, lighting environment, shot size, camera angle, lens, and camera movement. Official emotional mapping:
  - push-in → intimacy or tension
  - pull-out → scale or isolation
  - tracking shot → viewer moves with the subject
  - orbit → emphasizes the subject's importance
  - fixed camera → stillness and focus
- **Stylization:** named visual style (e.g. "cyberpunk," "line art illustration," "wasteland style").
- Official warning: most weak prompts cover entity and motion but skip aesthetic control and stylization, producing "a static camera in an undefined void."

**I2V (official):** the image already defines entity, scene, and style.

```text
[Motion] + [Camera movement]
```

- Describe what moves and how fast/intense ("quickly," "slowly," "gently," "violently").
- Explicit camera language: "camera pushes in," "camera moves left," "slow pan right," "orbit around the subject."
- To hold still, say **"fixed camera."** If camera direction is omitted, the model often invents a default move.
- Do not re-describe the subject at length; the image already did that.

**Official rewriter pattern (Wan2.2 GitHub, T2V-A14B):**

1. Optionally lead with a named style (only if the user provided one; put style first).
2. Add up to a few cinematic settings chosen from official menus (time of day, light source, intensity, angle, color tone, shot size, camera angle, composition). The English rewriter says **no more than four** such settings. Defaults if unspecified: daytime, medium or wide shot, center composition. If the user already described camera movement, do not add a competing camera-angle term. If the style is 2D/illustration and conflicts with live-action cinematography, skip movie-aesthetic add-ons.
3. Enrich appearance of subjects that are already present; **do not invent new subjects** (e.g. do not add a person to a landscape prompt).
4. Describe the motion process in concrete terms; if there is no action, add a plausible one (subject or background, e.g. leaves moving).
5. Avoid literary atmosphere language ("the frame is full of tension and formal atmosphere").
6. Output **only** the rewritten prompt (no "Rewritten prompt:" prefix).

### Length guidance

| Context | Limit / target |
| --- | --- |
| **Model Studio API, Wan 2.2 and 2.1** | Prompt **up to 800 characters**. Count is by character: each Chinese character **or** Latin letter counts as one. Excess is **truncated**. |
| **Model Studio API, Wan 2.5 / 2.6** | Up to **1,500 characters** (do not use this budget when targeting 2.2). |
| **negative_prompt** (all listed Wan versions) | Up to **500 characters**, then truncated. |
| **Official T2V rewriter** | Aim for about **60–200 words / 字**. |
| **Official I2V rewriter** | **100 words or fewer.** |
| **Community sweet spot (19 Nov 2026)** | About **80–120 words** for a constrained shot (HF forum, citing prompt guides). A tested Wan 2.2 set found **25–80 words** before later words are ignored (wan27.org). Do not raise the 800 / 500 character caps for this. |

For the rewriter app: when the target is Wan 2.2, stay inside **800 characters** after rewrite. Prefer the **80–120 word** band for T2V when the idea has enough to say, allow **25–80** for a simple shot, and do not pad toward 200. Keep I2V at **100 words or fewer**, then verify the character cap.

### Special syntax

- Natural cinematic English or Chinese; no required bracket command language (unlike Hailuo 02/2.3).
- Camera phrases the official guide and rewriter emphasize: camera pushes in / pulls out / moves left / moves right / pans / tilts / dolly / crane / handheld / zoom / orbit / tracking shot / fixed camera.
- Cloud APIs often expose `prompt_extend` / prompt rewriting (LLM expands short prompts; default **true** on Model Studio; increases latency; disable for precise control).
- Audio-in-prompt formulas are **not** official for 2.2. Later Wan versions add Voice + Sound effect + BGM descriptions.

### Chinese vs English

- Official API: prompts **may be in Chinese or English**.
- Official GitHub rewriter is dual: `T2V_A14B_ZH_SYS_PROMPT` / `I2V_A14B_ZH_SYS_PROMPT` output **Chinese**; `*_EN_SYS_PROMPT` output **must be fully English**, regardless of input language.
- Practical implication for the app: pick an output language (or follow the user's input language) and keep the rewriter consistent. Both languages are officially supported; do not claim one is required.

### Negative prompt support

**Yes**, as a separate API field `negative_prompt` (Chinese or English, max 500 characters). Official example:

`low resolution, error, worst quality, low quality, deformed, extra fingers, bad proportions`

This is not a substitute for a well-structured positive prompt. The rewriter should emit a clean positive prompt and, if useful, a short separate negative list — not mix negatives into the main Wan 2.2 prompt body.

### Do's and don'ts

**Do**

- Cover all five advanced dimensions for T2V finished shots.
- Be concrete about appearance, environment, and motion amplitude/speed/effect.
- Use professional camera vocabulary and specify "fixed camera" when stillness is wanted.
- For I2V, write only motion + camera, and preserve any user-supplied camera directions.
- Put a user-specified style first; do not invent a style.
- Keep 2.2 prompts inside the 800-character cap.

**Don't**

- Skip lighting, shot size, angle, lens, and camera movement on T2V (leads to a static camera in a void).
- Re-describe the I2V still as if it were T2V.
- Add subjects that were not in the user's prompt.
- Stack literary mood adjectives instead of visible facts.
- Apply Wan 2.5+ audio/sound formulas to Wan 2.2.
- Rely on the official cloud rewriter's safety behavior if the app is doing the rewrite itself; the GitHub system prompts include aggressive content-replacement rules. The app should use the **structure and length rules**, not copy those substitution policies.

### Short example of a strong prompt

**T2V (advanced):**

```text
Warm colors, side lighting, medium close-up, center composition. A young woman with wavy auburn hair in a vintage floral dress stands at a rain-streaked cafe window at dusk. Behind her, brass lamps and wooden tables recede into a softly blurred interior. She slowly lifts a ceramic cup, turns her head toward the glass, and exhales a faint thread of steam. The camera pushes in gently. Live-action, soft practical tungsten light mixed with cool blue residual daylight.
```

**I2V:**

```text
The woman slowly turns her head toward the window and smiles. Steam drifts from the cup. Camera pushes in; then holds a fixed camera.
```

### Sources

- Official prompt guide: https://www.alibabacloud.com/help/en/model-studio/text-to-video-prompt
- Official T2V API (2.1–2.6; prompt/negative length, CN/EN): https://www.alibabacloud.com/help/en/model-studio/legacy-wan-text-to-video-api-reference
- Official I2V first-frame API (same length rules): https://www.alibabacloud.com/help/en/model-studio/legacy-image-to-video-api-reference/
- Official Wan2.2 rewriter prompts: https://github.com/Wan-Video/Wan2.2/blob/main/wan/utils/system_prompt.py
- Official ComfyUI native Wan2.2 workflow (model table, not a separate prompt formula): https://docs.comfy.org/tutorials/video/wan/wan2_2
- fal.ai Wan 2.2 model pages (prompt + optional prompt expansion): e.g. https://fal.ai/models/fal-ai/wan/v2.2-a14b/text-to-video
- Replicate announcement / model pages: https://replicate.com/blog/wan-22
- Community synthesis that tracks the official formula and 800-character cap: https://vidogrid.com/wan-2-2-prompts

### Community prompt practice (19 Nov 2026)

Three later guides are folded into the Wan templates on top of the official formula. They do not change the API caps (800 positive, 500 negative).

**What they agree on**

- Wan spends more of the prompt on the opening words. Lead with the subject and the motion. A named style goes last, so it does not crowd out who does what. This is a deliberate change from the official rewriter's "put a user style first."
- Name the camera every time. With no camera line, Wan inserts a mild push-in. One shot size and one move. `fixed camera` (also called `static shot`) holds still. Do not combine aerial with close-up, or a locked camera with handheld, in the same moment.
- Motion needs speed, direction, and a body part. Chain two or three beats. Vague verbs ("dancing", "fighting") and "running fast" make Wan invent or smear the action.
- Say how many people are in frame, and say who must not enter. Image-to-video keeps the still for the opening frames and then drifts: extra people, a kiss, a new action.
- Few-step Wan (Lightning / Rapid, often 4–8 steps at CFG=1) mostly ignores a classic negative prompt. Put "they stay seated" and "nobody else enters" in the positive prompt. Keep blur, anatomy, and watermark cleanup in the negative field.
- LoRA trigger words change motion as well as looks. Copy them verbatim. That is what the app's protected-keyword list is for.

**Length.** Official T2V guidance remains 60–200 words, and official I2V remains 100 words or fewer. The HF forum answer (John6666, 13 Nov 2025, the post at `/170354/2`) cites an 80–120 word band for prompts that name the shot, the motion, and the constraints. wan27.org, from a large Wan 2.2 prompt set, puts the sweet spot at 25–80 words and says text past that is partly ignored. The templates aim at 80–120 when the idea needs the constraints, allow a shorter simple shot, and still stop at 800 characters.

**Vocabulary these guides treat as Wan-native.** Camera: static shot, slow pan left/right, slow tilt up/down, push in, pull back, zoom in, zoom out, tracking shot, arc shot, dolly, handheld, shallow depth of field, close-up, wide angle, aerial. Light: golden hour sunlight, long shadows, overcast lighting, soft/hard/side/edge/back/rim lighting, practical lighting, firelighting, fluorescent lighting, moonlighting, candlelit, neon on wet pavement. Motion: slowly, gradually, gently, briskly, quickly, suddenly, toward camera, away from camera, left to right. The MonsterMMORPG write-up is the official formula plus the official prompt dictionary (the author says it was taken from the Wan site). The templates keep a short form of that dictionary so the rewriter picks those words instead of synonyms.

**Negative list to start from, then add only what the shot needs.** Official: `low resolution, error, worst quality, low quality, deformed, extra fingers, bad proportions`. wan27.org also uses blurry, bad anatomy, extra limbs, fused fingers, watermark, text, logo, jpeg artifacts, oversaturated, overexposed, and, when relevant, inconsistent face, morphing features, plastic skin, shaky cam, or a crowded background.

**Sources**

- https://wan27.org/blog/wan-2-2-prompt-guide
- https://discuss.huggingface.co/t/how-to-get-the-most-out-of-prompts-for-wan-models/170354
- https://discuss.huggingface.co/t/how-to-get-the-most-out-of-prompts-for-wan-models/170354/2
- https://huggingface.co/blog/MonsterMMORPG/how-to-prompt-wan-models-full-tutorial-and-guide

---

## 2. MiniMax H3

### What "MiniMax H3" most likely is

**Conclusion:** The user almost certainly means the official **MiniMax-H3** video model (the Hailuo 03 / "H3" generation), **not** Hailuo 02 or Hailuo 2.3.

**Confidence: high (about 90–95%).** Reasons:

1. MiniMax officially announced **MiniMax H3** on **31 July 2026** as a general-purpose multimodal generation model: unified text/image/video/audio context, native stereo sound, up to **15 seconds** at up to **2K**. The post explicitly places it after Hailuo 01 and Hailuo 02: "From Hailuo 01 and Hailuo 02 to H3."
2. Official platform docs (read November 2026) say MiniMax's current video model is **MiniMax-H3**, and the Video Generation V2 API is described as the **"Hailuo-03" API**. Supported model IDs: `MiniMax-H3`, `MiniMax-H3-Highspeed`, `MiniMax-H3-Lite`, `MiniMax-H3-Max`.
3. Official GitHub org `MiniMax-AI/MiniMax-H3` ships a first-party skill, `h3-prompt-writing`, with a detailed English prompt-writing guide (`references/base-en.txt`, `ref-en.txt`).
4. Older models remain documented separately on the **v1** text-to-video API: `MiniMax-Hailuo-2.3`, `MiniMax-Hailuo-02`, `T2V-01-Director`, `T2V-01`. Those are real, but they are not named "H3."
5. Some secondary articles also call H3 "Hailuo 3.0" or "Hailuo 03." That alias is plausible given MiniMax's own "Hailuo-03 API" wording, but the **canonical product name and model ID are MiniMax-H3**.

If the app needs a fallback for users who still generate on older Hailuo endpoints, keep a **legacy Hailuo 02 / 2.3 / T2V-01-Director** profile (bracket camera commands, 2000-character cap). Default the "MiniMax H3" preset to **MiniMax-H3**.

### Versions this H3 guidance applies to

- Official prompt skill and video-generation guide for **MiniMax-H3** (and the Highspeed / Lite / Max variants, which share the V2 multimodal content API).
- Duration: integer **4–15 seconds** (docs also show a common default around 5–6s). Resolutions: **768P / 2K**. Prompt hard limit: **≤ 7,000 characters**.
- Modes: **T2VA** (text-to-video+audio), **I2VA** (first frame), **FL2VA** (first and last frames), **L2VA** (last frame only), **Ref2VA** (reference images/videos/audio). Official input caps include: first/last frames 0–2 images; references ≤ 9 images, ≤ 3 videos, ≤ 3 audio, mixed total ≤ 12 files.

### Recommended structure / formula templates

Official H3 rewrites use **named fields in a fixed order**, written in **English** (preserve dialogue, lyrics, and on-screen text in the original language).

**T2VA (text only) — official three core fields:**

```text
integrated_multimodal_description: [Shot 1] [Style], [composition]. [subjects, environment, actions]. The camera [motion type] [with small/large amplitude] [at slow/fast speed]. Speaker (S1) says: <d>[Language] verbatim dialogue</d> [Shot 2] At 00:05.000, the camera cuts to ...

overall_soundscape: [ambient + physical action + non-verbal human sounds across the whole clip]

non_diegetic_music: [instrumentation, tempo, rhythm, dynamics — or N/A]
```

**I2VA — instruction line first, then the three fields:**

```text
For the target video, at 0.00 seconds into the target video, <Picture 1> (from [Shot 1]) is fully referenced.

integrated_multimodal_description: ...
overall_soundscape: ...
non_diegetic_music: ...
```

Path inside the description: **first-frame anchor → action onset → continuous development → result or reaction.** Keep identity, clothing, colors, key objects, and spatial relationships consistent with the image.

**FL2VA:**

```text
How the reference pictures align with the target video — Picture 1 (from Shot 1) aligns with the 0.00-second mark of the target video; Picture 2 (from Shot N) aligns with the S.SS-second mark of the target video.

integrated_multimodal_description: ...
overall_soundscape: ...
non_diegetic_music: ...
```

`S.SS` is the video duration to **exactly two decimal places**. Prefer a **single shot** so the model can interpolate continuously; use multiple shots only if the user asked for them. Path: **first-frame state → observable intermediate changes → narrowing differences → last-frame state.**

**L2VA:**

```text
How the reference pictures align with the target video — <Picture 1> (from [Shot N]) aligns with the S.SS-second mark of the target video.

integrated_multimodal_description: ...
overall_soundscape: ...
non_diegetic_music: ...
```

The last-frame image belongs to the **final shot**, not automatically to Shot 1. Path: **plausible preceding state → action/transition → gradual convergence → last-frame landing.**

**Ref2VA** (full reference mode) uses a six-section format, in order: `subject_definitions`, `summary`, `retention_analysis`, `detailed_description`, `overall_soundscape`, `non_diegetic_music`, with consistent reference labels. Details are in `references/ref-en.txt` of the official skill.

A lighter "simple clip" order also appears in official/platform prose (subject → action → setting/light → camera → sound → style). That is fine for short casual prompts, but the **skill's named-field format is the official rewrite target**.

### Camera language (H3)

Official skill: write camera motion as **natural English inside the shot**, not as a stack of tags at the end. Complete expression = **motion type + optional amplitude + optional speed**. Omit amplitude/speed when they are medium/normal.

| Motion type | Meaning |
| --- | --- |
| Zoom In / Zoom Out | Focal length changes; camera body stays put |
| Push In / Pull Out | Camera body moves forward / backward |
| Pan Left / Pan Right | Camera stays, lens pivots horizontally |
| Truck Left / Truck Right | Camera translates horizontally |
| Tilt Up / Tilt Down | Camera stays, lens pivots vertically |
| Pedestal Up / Pedestal Down | Whole camera rises / lowers |
| Arc Shot | Camera moves in an arc around the subject |
| Tracking Shot | Camera follows a moving subject |
| Static Shot | Position and lens stay still |
| Shake Slightly / Shake Strongly | Camera shake |
| POV | Subject's point of view |
| Roll Clockwise / Roll Counterclockwise | Roll around the lens axis |
| Amplitude | `with small amplitude` / `with large amplitude` |
| Speed | `at slow speed` / `at fast speed` |

Official examples:

```text
The camera pushes in with small amplitude at slow speed toward the folded letter in her hands.
The camera pans right with large amplitude at fast speed, revealing the open doorway.
The camera holds a static shot as the runner exits the frame.
```

**Bracket shorthand still appears in official H3 product docs**, in a lighter form than Hailuo 02/2.3: "add camera motion instructions (e.g., [pan], [zoom], [static]) directly after key descriptions." Both are official; the writing skill prefers the full sentence form because it gives the model more to work with.

### Length guidance

- API hard cap: **7,000 characters**.
- Video length described in the prompt must **match the requested duration (4–15 seconds)**.
- Shot cut times must fall inside that duration.
- `overall_soundscape` and `non_diegetic_music` are short summaries (music: official skill says **1–3 English sentences**, or `N/A`).
- MiniMax also offers **H3-Context-IR**, an official async prompt enhancer that returns a structured prompt in `content.prompt` and does not itself generate video. The app can treat that as an optional upstream enricher, not as a replacement for knowing the format.

### Special syntax

- Shot labels: `[Shot 1]`, `[Shot 2]`, ...
- **Do not timestamp Shot 1.** Later shots start with a strictly increasing cut time, official form: `[Shot 2] At 00:03.500, the camera cuts to...`
- Cuts: `the camera cuts to`, `the shot cuts to`, `the shot transitions to`, etc. Cross-dissolve, fade, or wipe only when the user asks. A cut should introduce new information; for a mere distance/angle change, prefer camera motion.
- At the start of Shot 1, state style + initial composition. Common styles listed: Cinematic, live-action, 2D-animated, 3D CG, claymation, watercolor, vintage film.
- Speakers: stable IDs `(S1)`, `(S2)`, or compound `(S1,S2)`. Characters who never vocalize get no speaker ID. First appearance should establish type/age/gender/on-screen status, pitch, timbre, rate, or accent **outside** the dialogue block.
- Dialogue block: only language tag + verbatim user text inside the dialogue markers. Official examples use: `says: <d>[English] First batch of the morning.</d>` Do not translate or rewrite the spoken words. For voiceover, use the exact phrase `says in an off-screen voiceover`, then state that on-screen lips remain closed. If a line crosses a cut, mark continuity (`continues seamlessly across the cut`, etc.).
- Picture/reference labels such as `<Picture 1>` must stay consistent.
- `non_diegetic_music` is only music **the characters cannot hear**. Diegetic singing, radio, TV, or phone music belongs in `integrated_multimodal_description`. No abstract mood words in the music field.

### Negative prompt support

No separate `negative_prompt` field is documented on the official H3 V2 create API. Control unwanted elements in the structured description (and use `N/A` for music when there should be none). Do not assume a Hailuo-style negative box.

### Do's and don'ts

**Do**

- Match the timeline to the requested 4–15s duration.
- Prefer concrete visual and audio detail over "cinematic" or "beautiful."
- Write camera moves as in-shot sentences with type + amplitude + speed when useful.
- Keep speaker IDs, picture labels, and field names exact.
- Preserve original-language dialogue and visible text.
- For I2VA/FL2VA/L2VA, state how the keyframe connects to the timeline.

**Don't**

- Output a plot summary instead of a timed audiovisual description.
- Leave unresolved reference labels.
- Add a timestamp on Shot 1, or use cut times outside the duration.
- Stack camera labels only at the end of the prompt (skill guidance).
- Put audience-only music in the diegetic description, or vice versa.
- Invent dialogue wording; quote the user verbatim inside the dialogue block.

### Short example of a strong prompt (official-style T2VA)

```text
integrated_multimodal_description: [Shot 1] Live-action, cinematic, a medium-wide shot frames a baker opening the shutters of a small street bakery before sunrise. The camera pushes in with small amplitude at slow speed as the middle-aged baker with a calm, slightly raspy voice (S1) places a fresh loaf on the wooden counter and says: <d>[English] First batch of the morning.</d> [Shot 2] At 00:05.000, the camera cuts to a close-up of steam rising from the sliced bread while the baker's final words carry over from the previous shot.

overall_soundscape: Wooden shutters scrape open, followed by the soft thud of a loaf on the counter and a faint hiss of steam. Distant early-morning traffic stays in the background.

non_diegetic_music: A sparse acoustic guitar figure at a slow tempo, fading under the close-up.
```

### Legacy: Hailuo 02 / Hailuo 2.3 / T2V-01-Director (not H3, but easy to confuse)

These remain on `POST https://api.minimax.io/v1/video_generation`.

- Prompt cap: **2,000 characters**.
- Duration: typically **6s**, or **10s** only at 768P on Hailuo 02 / 2.3; 1080P is 6s only.
- `prompt_optimizer` defaults to **true**; set false for precise control.
- Official camera syntax is the 15 **bracket commands**, placed after the relevant action:

  `[Truck left]`, `[Truck right]`, `[Pan left]`, `[Pan right]`, `[Push in]`, `[Pull out]`, `[Pedestal up]`, `[Pedestal down]`, `[Tilt up]`, `[Tilt down]`, `[Zoom in]`, `[Zoom out]`, `[Shake]`, `[Tracking shot]`, `[Static shot]`

- Combine simultaneous moves in one bracket, max recommended **3**: `[Pan left,Pedestal up]`
- Sequence moves in order: `"...[Push in], then...[Push out]"`
- Natural language also works; official docs say explicit commands are more accurate.
- Official example: `A man picks up a book [Pedestal up], then reads [Static shot].`

The rewriter must **not** treat this bracket language as the primary H3 format. It is the official language for Hailuo 02 / 2.3 / T2V-01-Director, and only a light optional shorthand on H3.

### Sources

- Official H3 announcement: https://www.minimax.io/blog/minimax-h3
- Official video generation guide (H3 modes, 7000-char limit, 4–15s, bracket shorthand note, Context-IR): https://platform.minimax.io/docs/guides/video-generation
- Official V2 create API (model IDs, Hailuo-03 API): https://platform.minimax.io/docs/api-reference/video-generation-v2-create
- Official H3 prompt-writing skill: https://github.com/MiniMax-AI/MiniMax-H3/blob/main/skills/h3-prompt-writing/SKILL.md
- Official base prompt guide: https://raw.githubusercontent.com/MiniMax-AI/MiniMax-H3/refs/heads/main/skills/h3-prompt-writing/references/base-en.txt
- Official Hailuo 02/2.3 T2V API (2000 chars, 15 camera commands): https://platform.minimax.io/docs/api-reference/video-generation-t2v
- Secondary explainers (useful, not authoritative): https://the-ai-future.com/how-to/hailuo-ai-prompt-guide , https://minimax-h3-ai.com/blog/minimax-h3-prompt-guide/

---

## 3. Seedance (ByteDance video)

Prompt rules **change by generation**. The rewriter should select 1.0 vs 2.0 vs 2.5 (or 1.5 if the user is on that endpoint) before formatting.

Official recommended prompt length on the BytePlus **Create video generation task** API (applies across Seedance text input unless a later doc overrides it): **no more than 500 Chinese characters or 1,000 English words**. Longer prompts may cause the model to drop details. Third-party API mirrors sometimes cite a hard cap around 10,000 characters; prefer the official quality recommendation.

Chinese and English are officially supported on 1.0; 2.5 docs also emphasize native generation in more than 10 languages.

### 3.1 Seedance 1.0 Pro and 1.0 Pro Fast

Official guide: subject + action at the base; add camera, shot size, style, and multi-scene switches as needed.

**Templates:**

```text
# Basic T2V / I2V
[Subject] + [Action]

# Multi-action (chronological)
[Subject] [action 1], then [action 2], then [action 3].

# Multi-shot / scene change (official connector phrases)
[Shot A description]. camera/scene switch. [Shot B: if characters or setting change, re-describe them].
# Also used in official examples: "Switch to a medium shot:", "The camera switches to..."
```

Optional trailing **model text commands** (appended after the prompt):

```text
[prompt]  --resolution 1080p  --duration 5 --camerafixed false
```

Abbreviations in the official guide: `rs` (resolution), `dur` (duration seconds), `cf` (camera fixed).

**Camera language (official):** Tracking Shot, Pan Left/Right, Truck Left/Right, push in, pull out, follow, circle/orbit, zoom, and combinations for long takes. I2V prompts should focus on motion and camera, similar in spirit to Wan I2V.

**Multi-shot syntax:** 1.0's distinctive official phrase is **"camera/scene switch"** (or "The camera switches to..."). After each switch, re-describe new characters or settings. This is **not** the later `Shot 1 / Shot 2` or timestamp system.

**Negatives:** No dedicated negative-prompt field in the 1.0 prompt guide. Atmosphere may be described in positive or negative natural language for aesthetic control. Do not assume 2.5's "No subtitles / No BGM" contract exists here.

**Do's and don'ts:** Keep Chinese or English; put `--parameters` after the text, not mixed into prose; for I2V, prefer supported aspect ratios (1:1, 3:4, 4:3, 16:9, 9:16, 21:9); describe actions in time order; for multi-scene, use the official switch phrasing and re-specify changed subjects.

**Short example:**

```text
The ship cuts through the storm as lightning repeatedly splits the night sky. Switch to a medium shot: a captain stands on the deck, holding a vintage telescope and gazing into the distance. The camera slowly pushes forward as he stows the telescope, his expression resolute as he looks ahead.
```

**Source:** https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-1-0-pro-pro-fast

A separate official **Seedance 1.5 Pro** prompt guide exists at https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-1-5-pro. If the user's endpoint is 1.5, read that guide rather than forcing 1.0 or 2.x syntax.

### 3.2 Seedance 2.0 series

Official stance: the model is a multimodal director that splits a prompt into a **spatial layer** (what is in the frame) and a **temporal layer** (how things change). Write engineering-style instructions, not vague "cinematic" copy.

**Advanced official formula:**

```text
[precise subject] + [action details] + [scene/environment] + [lighting & color tone] + [camera movement] + [visual style] + [image quality] + [constraints]
```

**Multi-shot official template (preferred for complex clips):**

```text
[Optional reference bindings]
Use the girl in @Image 1 as the main character, use @Image 2 as the dormitory scene style reference, and refer to the camera movement in @Video 1.

Shot 1: [camera / transition]. [subject action and expression]. [position/space]. [audio].
Shot 2: ...
Shot 3: ...
```

Recommended shot-internal order: (1) camera movement or transition, (2) actions and expressions, (3) spatial change, (4) audio.

**Version caveat — timestamps:** Official 2.0 guide: support for precise timing such as `0–3 seconds` is **unstable**, and forcing duration per shot may produce abnormal results. Use **`Shot 1 / Shot 2 / Shot 3` only** and let the model pace the clip. This is the opposite of 2.5.

**Reference syntax:** Bind assets by upload order, e.g. `@Image 1`, `@Video 1`, `@Audio 1`. Define subjects with 2–3 stable features (`Define the woman wearing a red dress and a straw hat in Image 1 as Subject 1`) and reuse that label. For edit/extend tasks, refer to the video directly; **do not** write "reference [Video]" or the task may be misclassified as an omni-reference job.

**Camera:** Standard terms work (medium shot, close-up, wide shot, slow push-in, smooth lateral tracking, fixed shot). **Specify only one camera movement per shot.** Do not ask for push, pull, pan, and move in the same shot.

**Actions:** Name body parts, range, speed, and force. Prefer slow, gentle, continuous motion; avoid packing high-burst action. Externalize emotion as visible behavior ("shoulders trembling, eyes reddening"), not "very sad."

**Constraints (official templates, written in the prompt):**

- "keep it subtitle-free" / "avoid generating any text or subtitles"
- "do not generate a logo"
- "do not generate a watermark"

**Do's and don'ts:** Be concise; avoid contradictory subject traits; do not use 2.5-style timestamps on 2.0; one camera move per shot; keep labels stable; include quality/style/constraints.

**Short example (official positive storyboard):**

```text
Shot 1: Side shot of a street alley; the man slowly starts running, with a sense of rapid breathing.
Shot 2: The man knocks over a fruit stand; the camera shakes quickly and gives a close-up of the man's frightened face.
Shot 3: The man climbs over a low wall and disappears; the camera slowly pulls back and freezes on the empty street.
```

**Sources:** https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-2-0-prompt-guide  
ByteDance Seed launch post: https://seed.bytedance.com/zh/blog/official-launch-of-seedance-2-0  
Replicate model page: https://replicate.com/bytedance/seedance-2.0

### 3.3 Seedance 2.5 and 2.5 flash

Current official flagship formula (BytePlus guide, also republished/adapted on fal.ai and attributed to BytePlus). Clips up to **30 seconds**. Stronger instruction following, professional edit/extend controls, native generation in 10+ languages.

Reference caps: **Seedance 2.5** — 30 images, 10 videos, 10 audio; **2.5 flash** — 10 / 5 / 5. Official materials sometimes mention a combined ceiling around 50 assets; stay inside the per-type caps.

**Official four-part prompt structure:**

```text
[1. Asset referencing for R2V]
Image 1: [who/what and what to take from it].
Video 1: [motion / camera to follow].
Audio 1: [voice / music role].

[2. One-sentence summary]
[Subject] + [Location] + [Event] + [Genre/Style] + [Camera movement]

[3. Detailed plot — Shot N and/or timestamps]
Shot 1 (0-3s): [visuals, camera, actions, dialogue, SFX]
Shot 2 (3-8s): ...

[4. Additional notes / constants]
[elements that must stay consistent: angle, environment, sound, style, "No subtitles", etc.]
```

**Timestamps (2.5 only; this is the headline difference vs 2.0):**

- Use **1-second intervals** as the basic unit.
- Keep the timeline continuous; avoid gaps such as `0-3s ... 5-6s`.
- Too little plot in a range → the model improvises; too much → extra cuts or dropped beats.
- Do not use timestamps to control high-frequency actions ("shake your head three times per second").
- Official forms:
  - Intervals: `0-3 seconds...3-7 seconds...7-15 seconds` or `[1s-4s]....[4s-8s]`
  - Time points: `At the 5-second mark, ...` / `Quick left sideways transition at the 5-second mark.`
  - Relative time: `After 3 seconds, ...` / `The frame freezes for 1 second after ...`
- `Shot N` still works on 2.5 and can be combined with timestamps: `Shot 5 (14-18s): ...`

**Camera language:** Write standard terms directly — shot size (extreme wide / wide / medium / medium close-up / close-up), movement (push in, pull out, pan, track, follow, orbit, dive, pull back, tilt up, handheld shake), angle (low angle, overhead, first-person). Advanced techniques (one-shot/long take, Hitchcock/dolly zoom, aerial, FPV, bullet time, speed ramp) can also be named. For niche terms, use **[term + descriptive explanation]**, e.g. "Rack focus: the focus shifts smoothly; the trees that were originally clear in the foreground become blurred, while the character in the background gradually becomes clear." For transitions, give **both timing and method**: "At the 5-second mark, the camera quickly transitions leftward using a left wipe combined with a natural dissolve."

**Reference mapping:** Number assets by upload order (`Image 1 / Video 1 / Audio 1` or `@Image 1`). Bind each asset in the text. Do not rely on text written *on* a reference image ("John" scrawled on a photo) as the only identity. If the reference already contains the motion/camera you want, say "strictly refer to..." and do not re-describe every gesture.

**Editing / extend / first-last frame triggers (official):**

- Edit: include words such as edit video, add, insert, remove, delete, modify, replace, change to. Example: `Change the man's action from drinking coffee to mopping the floor from 4-6 seconds in Video 1, and leave the rest of the content unchanged.`
- Extend: extend forward/backward, continue, continue from, extend the story.
- First/last frames: prefer API roles `first_frame` / `last_frame` (locks aspect ratio to the first image). Or designate in text: `Image 3 is the first frame, and Image 5 is the last frame` (similar, not pixel-exact).

**Negative control (official, limited but real):**

- Subtitles: "Do not add subtitles." / "No subtitles."
- Audio: "No BGM; generate only environmental sounds and action sounds." / "No audio."
- Prefer **positive** description for everything else.

fal.ai's BytePlus-authored 2.5 guide restates a compact core formula that is consistent with the above:

```text
<Subject> performs <primary action or event> in <scene and environment>.
The visuals feature <visual style>.
Use <shot size, camera angle, camera movement, or cuts>.
Audio includes <dialogue, ambience, sound effects, or music>.
```

**Do's and don'ts:** Use timestamps only on 2.5; keep ranges continuous and realistically paced; map every reference in prose; write "No subtitles" / "No BGM" when needed; for on-screen text or logos, official and community notes suggest supplying a reference image or spelling carefully — do not expect frame-perfect typography from text alone; timestamps allocate time, they are not frame-accurate edit points.

**Short example (official-style 2.5, no refs):**

```text
Realistic nature documentary style, natural lighting and shadows. On a warm afternoon, on a grassy slope in the forest, a chubby panda cub rolls down the hill.

The panda has fluffy, realistic black-and-white fur, a small round body, and clumsy, adorable movements. Low-angle medium-wide shot with a slight handheld feel, keeping the panda in frame.

0s-3s: A panda cub lies on a green grassy slope and begins to slowly roll sideways down the slope. Sunlight filters through the trees from the upper left.
3s-8s: The panda rolls toward the lower right, stops, shifts onto its belly, turns its face toward the camera, and makes a soft humming sound.

Natural environmental audio only, including wind, rustling grass, and the soft plop of the panda rolling. No subtitles. No BGM.
```

**Sources:**

- Official 2.5 prompt guide: https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-2-5-prompt-guide
- Official 2.5 skill installer (prompt optimizer): documented on that same page (`npx ... --skill sd25-pe`)
- fal.ai / BytePlus 2.5 documentation guide: https://fal.ai/learn/devs/how-to-use-seedance-2-5
- Community walkthrough that cites the Nov 2026 official 2.5 vs 2.0 timestamp split: https://aliteq.com/seedance-prompts

---

## 4. Seedream (ByteDance image)

### Versions this section applies to

- **Seedream 3.0:** older T2I generation. Official 4.0/4.5 guide contrasts it directly: 3.0 needed more description and tended toward washed-out images. Do not use 4.x "keep it concise" advice as if it applied unchanged to 3.0; 3.0 generally wanted **richer** description.
- **Seedream 4.0 and 4.5:** official joint prompt guide (BytePlus, updated **13 Nov 2026**). This is the best official "how to write the prompt" document for the series' modern natural-language style.
- **Seedream 5.0 lite, 5.0 pro, 5.0 flash:** current generation on BytePlus / fal / Replicate as of 2026. Basic T2I / I2I / multi-image prompting follows the same natural-language + quoted-text practices. **5.0 pro / flash** add official **interactive coordinate editing** and layer decomposition (up to 16 layers). **5.0 lite** still supports sequential/multi-image generation like 4.x; 5.0 pro/flash docs say `sequential_image_generation` is not supported on those two.
- **Seedream 5.5:** already listed as new in the official BytePlus image-generation API (richer layer decomposition, up to 99 layers, rich-text OCR, native RGBA). Prompt-language and length recommendations on that API page cover 5.5 together with 5.0 pro/flash.

### Recommended structure / formula templates

**Official 4.0 / 4.5 general formula (natural language, not keyword stacks):**

```text
[subject] + [action] + [environment], [style / color / lighting / composition if aesthetics matter]
```

If there is a use case, **state the image type and purpose** first (poster, logo, product hero, infographic, UI, storyboard).

**Text-to-image template:**

```text
[Optional purpose: Design a poster / product photo / infographic for ...]
[Subject and actions in coherent sentences].
[Environment, layout, left-to-right or top-to-bottom placement].
[Style, lighting, materials, composition].
On-image text: "[exact string]".
```

**Image editing template:**

```text
[Add / Remove / Replace / Modify] [specific target, not "that one"].
[Describe the end state].
Keeping [pose / identity / background / typography / layout] unchanged.
```

**Reference-based generation template:**

```text
Reference target: extract [character design / product / style] from [Image 1].
Generated scene: [scene, layout, placement, size, camera/lighting].
The overall style should remain consistent with the reference. Keep [X] unchanged.
```

**Multi-image input template:**

```text
Replace the subject in Image 1 with the subject from Image 2.
Dress the character in Image 1 with the outfit from Image 2.
Apply the style of Image 2 to Image 1.
```

**Multi-image output (4.0 / 4.5 / 5.0 lite):** trigger with "a series", "a set", or an explicit count (`Generate four film storyboard images...`, `Generate seven mobile phone wallpapers...`).

**5.0 pro / flash interactive editing (official coordinate syntax):**

Normalized coordinates **0–999**, top-left `0,0`, bottom-right `999,999`.

- Point: `x y`
- Bounding box: `x1 y1 x2 y2` (top-left, then bottom-right)

```text
Use the subject in Image 2 118 331 933 871 to replace the subject in Image 1 179 283 796 986.
Replace the person on the left in Image 1 120 180 640 760 with a robot.
Replace the area Image 1 120 180 640 760 with a garden, and keep the area Image 1 700 120 920 360 unchanged.
```

If a box contains several subjects, name the target ("the person on the left", "the cat wearing a hat").

fal.ai's independent **Seedream 5.0 Lite** testing guide (not ByteDance official, but widely used on fal) uses:

```text
[Subject] > [Setting] > [Style] > [Lighting] > [Technical]
```

and recommends keeping prompts **under ~200 words**, quoting rendered text, pairing hex with color names, and using explicit spatial words (left, right, foreground, between).

### Length guidance

Official BytePlus image-generation API (covers Seedream 4.x, 5.0 lite/pro/flash, and 5.5):

> Use **no more than 300 Chinese characters or 600 English words**. Excessively long prompts may scatter information so the model focuses only on major elements and drops details.

This is a **quality recommendation**, not a clearly stated hard cap in the official API paragraph above. Third-party model cards sometimes list hard caps around 4,000–5,000 characters; the rewriter should still aim at the official 300 / 600 guidance, and be even more concise on 4.0+ than on 3.0.

### Special syntax

- **On-image text must be wrapped in double quotation marks.** Official 4.0–4.5 example: `Generate a poster with the title "Seedream 4.5".` Avoid: `Generate a poster titled Seedream 4.5.`
- Specify placement, font/style, contrast, and layout when typography matters. 4.0+ is officially strong at formulas, tables, chemical structures, charts, posters, and UI text.
- 5.0 pro/flash also natively generate text in many languages beyond Chinese and English (official list includes Russian, Arabic, Filipino, Thai, Turkish, Korean, Malay, Spanish, Portuguese, Indonesian, French, German, Vietnamese, and Japanese).
- Editing: no vague pronouns. Name the target. State what stays unchanged.
- Visual markers (arrows, doodles, bounding boxes) can designate regions in 4.x editing; 5.0 pro/flash formalize this as normalized coordinates in the prompt.
- Sketch-to-image: if the sketch contains text annotations, say **"Generate based on the text in the image"** (or "according to the textual annotations in the sketch"). Explicitly drop unwanted sketch edges or labels: "Do not include any text or hand-drawn edges from the original sketch."

### Negative prompt support

No separate official `negative_prompt` field is described in the 4.0–4.5 prompt guide or the image-generation API excerpt reviewed for this research. Unwanted elements are handled in the positive prompt ("Do not include any text or hand-drawn edges...", "keeping its pose unchanged", "keep the area ... unchanged"). Do not invent a Stable-Diffusion-style negative box unless a specific host documents one.

### Do's and don'ts

**Do**

- Write coherent natural-language sentences: subject + action + environment.
- State the use case (logo, poster, product shot, storyboard, UI).
- Put every required on-image string in `"double quotes"`.
- For 4.0 / 4.5 / 5.x, prefer **concise and precise** over ornate stacked adjectives. Official wording: 4.0/4.5 understand prompts better than 3.0, "and the images are no longer washed out."
- For knowledge-dense images, use precise technical terms and specify format, layout, and style.
- For edits and references, name the source image, the extracted element, the new scene, and what must not change.
- For 3.0, allow a more detailed description than you would send to 4.x/5.x.

**Don't**

- Write comma-separated tag soup: `Girl, umbrella, tree-lined street, oil painting texture.`
- Leave required text unquoted.
- Use vague pronouns in edits ("Put that one in pink clothes").
- Overwrite identity, typography, or pose by omission — say what stays unchanged.
- Over-lengthen 4.x/5.x prompts until details get dropped.
- Assume 5.0 pro/flash sequential "generate a set of 15 images" behaves like 4.5 / 5.0 lite; check the model.

### Short example of a strong prompt (4.5/5.x T2I with text)

```text
Design a product poster for a ceramic coffee company. A handmade red ceramic mug with a slightly uneven glaze sits on a weathered oak table, steam curling through warm morning light from a window on the left. Food photography, shallow depth of field, soft side light. The poster title is "First Batch", and a small caption at the bottom reads "Roasted at dawn". Clean editorial layout with space at the top for the title. No extra logos.
```

**Editing example (official style):**

```text
Dress the tallest panda in pink Peking Opera costume and headgear, keeping its pose unchanged.
```

### Sources

- Official 4.0–4.5 prompt guide: https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-4-0-5-0-prompt-guide (also https://docs.byteplus.com/en/docs/ModelArk/seedream-4-0-5-0-prompt-guide)
- Official image-generation API (length recommendation, language support, 5.0/5.5 capabilities): https://docs.byteplus.com/docs/en/modelark/image-generation-api.md
- Official Seedream 4.0 launch (text rendering): https://seed.bytedance.com/en/blog/seedream-4-0-officially-released-beyond-drawing-into-imagination
- Official 5.0 pro/flash tutorial: https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-5-0-pro
- Official 5.0 pro/flash interactive editing guide: https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-5-0-pro-editing-guide
- fal.ai 5.0 Lite prompting guide (independent tests): https://blog.fal.ai/seedream-5-0-lite-prompting-guide/
- Replicate Seedream 5.0 lite: https://replicate.com/bytedance/seedream-5-lite

---

## 5. App note: OpenRouter and Ollama APIs

Yes. A single OpenAI-compatible chat-completions client can talk to **both** backends by switching **base URL**, **API key**, and **model id**. That is the straightforward architecture for the overlay rewriter (LLM in, rewritten prompt text out). Neither OpenRouter nor Ollama is the Wan/H3/Seedance/Seedream generator; they only host the rewriter model.

### OpenRouter

- Official chat-completions endpoint: **`POST https://openrouter.ai/api/v1/chat/completions`**
- OpenAI-compatible base URL: **`https://openrouter.ai/api/v1`**
- Auth: `Authorization: Bearer $OPENROUTER_API_KEY`
- Body: standard `model` + `messages` (system/user/assistant). Optional ranking headers: `HTTP-Referer`, `X-OpenRouter-Title`.
- Model IDs are prefixed, e.g. `openai/gpt-4o-mini`, `anthropic/claude-3.5-sonnet`.
- Official docs: https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion and https://openrouter.ai/docs/guides/features/compatibility

### Ollama (local)

- Default local server: **`http://localhost:11434`**
- Native API: **`POST http://localhost:11434/api/chat`**  
  Body: `{ "model": "...", "messages": [ { "role": "user", "content": "..." } ], "stream": false }`  
  Reply text is in `message.content`. Docs: https://docs.ollama.com/api/chat
- OpenAI-compatible API: **`POST http://localhost:11434/v1/chat/completions`**  
  Base URL for official OpenAI SDKs: **`http://localhost:11434/v1`** (trailing slash optional in examples).  
  `api_key` is required by the OpenAI SDK but **ignored** locally (common placeholder: `"ollama"`).  
  Docs: https://docs.ollama.com/api/openai-compatibility
- For one client that also speaks OpenRouter, prefer Ollama's **`/v1/chat/completions`** path so the request/response shape stays `choices[0].message.content`.

### Practical implication (no code)

Use one OpenAI SDK or HTTP helper:

| Backend | base URL | key | model field |
| --- | --- | --- | --- |
| OpenRouter | `https://openrouter.ai/api/v1` | OpenRouter key | provider-prefixed slug |
| Ollama | `http://localhost:11434/v1` | any placeholder | local tag, e.g. `llama3.2` |

The rewriter system prompt should change with the **target generator** (Wan 2.2 vs H3 vs Seedance 1.0/2.0/2.5 vs Seedream 4.x/5.x), not with the LLM backend.

---

## 6. Implications for the planned rewriter app (planning only)

When the user later says "go," these research points should drive the product, not a single generic "cinematic prompt":

1. **Target profile is mandatory.** Seedance 2.0 vs 2.5 (timestamps), Wan 2.2 vs later Wan (audio), H3 vs Hailuo 02/2.3 (named fields vs `[Pan left]`), Seedream 3.0 vs 4.x/5.x (verbose vs concise) will otherwise produce the wrong full text.
2. **Output is the full prompt ready to paste**, including official field names, shot labels, quoted poster text, camera sentences, and optional separate Wan `negative_prompt`.
3. **Enforce length caps in the rewriter instructions:** Wan 2.2 800 characters (T2V ~60–200 words, I2V ≤100 words); H3 match 4–15s and stay under 7,000 characters; Seedance prefer ≤500 Chinese characters or ≤1,000 English words; Seedream prefer ≤300 Chinese characters or ≤600 English words.
4. **Mode matters:** T2V vs I2V vs first-last vs reference changes the formula for Wan, H3, and Seedance.
5. **OpenRouter and local Ollama can share one chat-completions client.**

---

## 7. Source index (official first)

### Wan
- https://www.alibabacloud.com/help/en/model-studio/text-to-video-prompt
- https://www.alibabacloud.com/help/en/model-studio/legacy-wan-text-to-video-api-reference
- https://www.alibabacloud.com/help/en/model-studio/legacy-image-to-video-api-reference/
- https://github.com/Wan-Video/Wan2.2/blob/main/wan/utils/system_prompt.py
- https://docs.comfy.org/tutorials/video/wan/wan2_2
- https://fal.ai/models/fal-ai/wan/v2.2-a14b/text-to-video
- https://replicate.com/blog/wan-22
- https://vidogrid.com/wan-2-2-prompts
- https://wan27.org/blog/wan-2-2-prompt-guide
- https://discuss.huggingface.co/t/how-to-get-the-most-out-of-prompts-for-wan-models/170354
- https://discuss.huggingface.co/t/how-to-get-the-most-out-of-prompts-for-wan-models/170354/2
- https://huggingface.co/blog/MonsterMMORPG/how-to-prompt-wan-models-full-tutorial-and-guide

### MiniMax H3 and legacy Hailuo
- https://www.minimax.io/blog/minimax-h3
- https://platform.minimax.io/docs/guides/video-generation
- https://platform.minimax.io/docs/api-reference/video-generation-v2-create
- https://github.com/MiniMax-AI/MiniMax-H3/blob/main/skills/h3-prompt-writing/SKILL.md
- https://raw.githubusercontent.com/MiniMax-AI/MiniMax-H3/refs/heads/main/skills/h3-prompt-writing/references/base-en.txt
- https://platform.minimax.io/docs/api-reference/video-generation-t2v

### Seedance
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-1-0-pro-pro-fast
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-1-5-pro
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-2-0-prompt-guide
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedance-2-5-prompt-guide
- https://docs.byteplus.com/docs/en/modelark/create-video-generation-task-api.md
- https://fal.ai/learn/devs/how-to-use-seedance-2-5
- https://aliteq.com/seedance-prompts

### Seedream
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-4-0-5-0-prompt-guide
- https://docs.byteplus.com/docs/en/modelark/image-generation-api.md
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-5-0-pro
- https://ai.byteplus.com/ark/region:ap-southeast-1/docs/seedream-5-0-pro-editing-guide
- https://seed.bytedance.com/en/blog/seedream-4-0-officially-released-beyond-drawing-into-imagination
- https://blog.fal.ai/seedream-5-0-lite-prompting-guide/

### Rewriter backends
- https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion
- https://openrouter.ai/docs/guides/features/compatibility
- https://docs.ollama.com/api/chat
- https://docs.ollama.com/api/openai-compatibility

---

## 8. Field notes: Seedream in practice

These are not from ByteDance's documentation. They come from generating hundreds of story-scene pictures with Seedream 5.0 (with one reference photo per character) in a story app, and from what went wrong there. They sit on top of section 4, not in place of it.

- **Labelled lines are fine.** `Action:` / `Subjects:` / `Outfits:` / `Background:` / `Lighting:` / `Camera:` / `Style:`, each a full sentence or two, read as natural language to Seedream and are much easier for a person to edit than one paragraph. Tag soup inside a line is still tag soup.
- **Lead with the action.** Prompts that spend their words on how people look (face, hair, body) come back as posed portraits and lose the event. Put the moment first: who does what, to whom.
- **One pose per person.** Several actions in one picture ("turns, waves, picks up the cup") merge bodies, tangle limbs and add hands. Pick one moment, place both hands or frame them out, and prefer a close-up or half-body shot when the hands do not matter.
- **Do not re-describe referenced faces.** When a person has a reference photo, describing their face or hair in words pulls the result away from the photo. Bind by number ("the woman from Image 1") and describe pose, action and clothes only.
- **A portrait reference brings its clothes along.** Unless the prompt names the scene's outfit in words and says it replaces the photo's clothes, Seedream copies the photo's clothes, including modern clothes into a period scene. State the era for everyone ("Tang-dynasty hanfu, no modern clothes").
- **One reference per person.** Extra photos of the same person cost more on most hosts and did not improve likeness.
- **First-person point of view.** "First-person point of view" with the viewer out of frame (hands at most) works. Naming the viewer ("seen through Mara's eyes") does not help: the model does not know who that is and sometimes draws them.
- **Say the time and light from the scene.** "Late night, lantern light" beats "cinematic lighting", which drifts to golden hour.
- **No text unless asked.** Ending the style with "No text, captions or watermarks." stops stray captions on story scenes.
- **Video (Seedance) is stricter still.** A busy clip prompt is where people merge most. One simple movement per clip, at most two people moving, no "then", and well under 100 words worked far better than a detailed paragraph.
