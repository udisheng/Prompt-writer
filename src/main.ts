import {
  api,
  errorText,
  inTauri,
  type HistoryEntry,
  type ModelInfo,
  type RewriteResult,
  type SettingsView,
  type TemplateSummary,
} from "./api.ts";

const FAMILIES = [
  { id: "wan22", label: "Wan 2.2", options: [
    { id: "wan22-t2v", label: "Text to video" },
    { id: "wan22-i2v", label: "Image to video" },
  ]},
  { id: "minimax-h3", label: "MiniMax H3", options: [{ id: "minimax-h3", label: "Text to video" }] },
  { id: "seedance", label: "Seedance", options: [
    { id: "seedance-1.0", label: "1.0" },
    { id: "seedance-2.0", label: "2.0" },
    { id: "seedance-2.5", label: "2.5" },
  ]},
  { id: "seedream", label: "Seedream", options: [
    { id: "seedream-generate", label: "Generate" },
    { id: "seedream-edit", label: "Edit" },
  ]},
];

const HINTS: Record<string, string> = {
  "wan22-t2v": "Subject, motion, then camera and light. About 80–120 words, and the positive prompt must stay within 800 characters.",
  "wan22-i2v": "Count, motion, one camera, and what must not happen. “fixed camera” holds a still shot. 100 words.",
  "minimax-h3": "Labelled shots, a soundscape, and a score. Duration is part of the prompt, from 4 to 15 seconds.",
  "seedance-1.0": "Subject plus action. Scene changes use “camera/scene switch”. Duration is a trailing --duration flag.",
  "seedance-2.0": "Shot 1, Shot 2, one camera move each. Bind references as @Image 1. Do not use timestamps.",
  "seedance-2.5": "Asset map, summary, timestamped shots, and constants. Clips can run up to 30 seconds.",
  "seedream-generate": "Labelled lines: Action, Subjects, Outfits, Background, Lighting, Camera, Style. One pose per person. On-image text goes in double quotes.",
  "seedream-edit": "Name what changes and what must stay unchanged. Keep faces unchanged. Keep it short.",
};

const ui = {
  banner: must<HTMLParagraphElement>("banner"),
  note: must<HTMLParagraphElement>("desktop-note"),
  family: must<HTMLSelectElement>("family"),
  variants: must<HTMLDivElement>("variants"),
  variantWrap: must<HTMLDivElement>("variant-wrap"),
  variantLabel: must<HTMLSpanElement>("variant-label"),
  durationWrap: must<HTMLDivElement>("duration-wrap"),
  duration: must<HTMLInputElement>("duration"),
  durationHint: must<HTMLSpanElement>("duration-hint"),
  hint: must<HTMLParagraphElement>("mode-hint"),
  idea: must<HTMLTextAreaElement>("idea"),
  ideaCount: must<HTMLElement>("idea-count"),
  rewrite: must<HTMLButtonElement>("rewrite"),
  outputLabel: must<HTMLSpanElement>("output-label"),
  outputCount: must<HTMLElement>("output-count"),
  output: must<HTMLTextAreaElement>("output"),
  copyOutput: must<HTMLButtonElement>("copy-output"),
  negativeCard: must<HTMLElement>("negative-card"),
  negativeCount: must<HTMLElement>("negative-count"),
  negative: must<HTMLTextAreaElement>("negative"),
  copyNegative: must<HTMLButtonElement>("copy-negative"),
  copyBoth: must<HTMLButtonElement>("copy-both"),
  warnings: must<HTMLUListElement>("warnings"),
  commandHint: must<HTMLDivElement>("command-hint"),
  protectedActions: must<HTMLDivElement>("protected-actions"),
  appendProtected: must<HTMLButtonElement>("append-protected"),
  keywords: must<HTMLTextAreaElement>("keywords-editor"),
  keywordsPath: must<HTMLParagraphElement>("keywords-path"),
  historyList: must<HTMLDivElement>("history-list"),
  historyEmpty: must<HTMLDivElement>("history-empty"),
  clearHistory: must<HTMLButtonElement>("clear-history"),
  apiKey: must<HTMLInputElement>("api-key"),
  keyStatus: must<HTMLParagraphElement>("key-status"),
  orModel: must<HTMLInputElement>("or-model"),
  orModels: must<HTMLDataListElement>("or-models"),
  orStatus: must<HTMLParagraphElement>("or-model-status"),
  ollamaUrl: must<HTMLInputElement>("ollama-url"),
  ollamaModel: must<HTMLInputElement>("ollama-model"),
  ollamaModels: must<HTMLDataListElement>("ollama-models"),
  ollamaStatus: must<HTMLParagraphElement>("ollama-status"),
  hotkey: must<HTMLInputElement>("hotkey"),
  record: must<HTMLButtonElement>("record-hotkey"),
  hotkeyStatus: must<HTMLParagraphElement>("hotkey-status"),
  defaultTemplate: must<HTMLSelectElement>("default-template"),
  temperature: must<HTMLInputElement>("temperature"),
  tempValue: must<HTMLElement>("temp-value"),
  templatesPath: must<HTMLParagraphElement>("templates-path"),
  templateFile: must<HTMLSelectElement>("template-file"),
  templateEditor: must<HTMLTextAreaElement>("template-editor"),
};

const state: {
  settings: SettingsView | null;
  templates: TemplateSummary[];
  templateId: string;
  busy: boolean;
  recording: false | ((event: KeyboardEvent) => void);
  result: RewriteResult | null;
  commandOutput: boolean;
} = {
  settings: null,
  templates: [],
  templateId: "wan22-t2v",
  busy: false,
  recording: false,
  result: null,
  commandOutput: false,
};

const COMMANDS = [
  { label: "/show key", detail: "List protected keywords" },
  { label: "/add key ", detail: "Add words or phrases, separated by commas" },
  { label: "/remove key ", detail: "Remove by text or by the number from /show key" },
  { label: "/clear key", detail: "Ask before deleting the list" },
  { label: "/clear key confirm", detail: "Delete every protected keyword" },
  { label: "/help", detail: "List these commands" },
];

function must<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Missing #${id}`);
  return node as T;
}

function familyOf(id: string): string {
  return FAMILIES.find((family) => family.options.some((option) => option.id === id))?.id ?? "wan22";
}

function templateOf(id: string): TemplateSummary | undefined {
  return state.templates.find((template) => template.id === id);
}

function durationBounds(id: string): { min: number; max: number } | null {
  if (id === "minimax-h3") return { min: 4, max: 15 };
  if (id === "seedance-2.5") return { min: 4, max: 30 };
  if (id.startsWith("seedance-")) return { min: 4, max: 15 };
  return null;
}

function isWan(id: string): boolean {
  return templateOf(id)?.output === "wan_pair" || id.startsWith("wan22");
}

function showBanner(message: string) {
  ui.banner.hidden = false;
  ui.banner.textContent = message;
}

function clearBanner() {
  ui.banner.hidden = true;
  ui.banner.textContent = "";
}

function flash(message: string) {
  showBanner(message);
  window.setTimeout(() => {
    if (ui.banner.textContent === message) clearBanner();
  }, 2200);
}

async function copyText(text: string) {
  if (!text.trim()) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = document.createElement("textarea");
    area.value = text;
    area.style.position = "fixed";
    area.style.left = "-1000px";
    document.body.appendChild(area);
    area.select();
    document.execCommand("copy");
    area.remove();
  }
  flash("Copied.");
}

function fillFamilies() {
  ui.family.replaceChildren();
  for (const family of FAMILIES) {
    const option = document.createElement("option");
    option.value = family.id;
    option.textContent = family.label;
    ui.family.append(option);
  }
}

function renderVariants() {
  const family = FAMILIES.find((item) => item.id === ui.family.value) ?? FAMILIES[0];
  ui.variantLabel.textContent = family.id === "seedance" ? "Version" : family.id === "seedream" ? "Task" : "Mode";
  ui.variants.replaceChildren();
  ui.variantWrap.hidden = family.options.length < 2;
  if (!family.options.some((option) => option.id === state.templateId)) {
    state.templateId = family.options[0].id;
  }
  for (const option of family.options) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = option.label;
    button.setAttribute("aria-pressed", String(option.id === state.templateId));
    button.addEventListener("click", () => {
      state.templateId = option.id;
      state.result = null;
      state.commandOutput = false;
      ui.output.value = "";
      ui.negative.value = "";
      renderVariants();
      renderOutput();
    });
    ui.variants.append(button);
  }
  const bounds = durationBounds(state.templateId);
  ui.durationWrap.hidden = bounds == null;
  if (bounds) {
    ui.duration.min = String(bounds.min);
    ui.duration.max = String(bounds.max);
    const current = Number(ui.duration.value);
    if (!Number.isInteger(current) || current < bounds.min) ui.duration.value = String(bounds.min);
    if (current > bounds.max) ui.duration.value = String(bounds.max);
    ui.durationHint.textContent = `seconds, ${bounds.min}–${bounds.max}`;
  }
  ui.hint.textContent = HINTS[state.templateId] ?? "";
  ui.negativeCard.hidden = !isWan(state.templateId);
  ui.outputLabel.textContent = isWan(state.templateId) ? "Positive prompt" : "Prompt";
}

function applyTemplateId(id: string) {
  state.templateId = id;
  ui.family.value = familyOf(id);
  renderVariants();
}

function countClass(count: number, max: number | null): string {
  if (max == null) return "count";
  if (count > max) return "count over";
  if (count > max * 0.9) return "count warn";
  return "count";
}

function countText(count: number, max: number | null, kind: "model" | "suggested" | null, unit: string): string {
  if (max == null) return `${count} ${unit}`;
  const name = kind === "suggested" ? "suggested" : "limit";
  return `${count.toLocaleString()} / ${max.toLocaleString()} ${name}`;
}

function renderCommandHint() {
  const typed = ui.idea.value.trimStart().toLowerCase();
  ui.commandHint.replaceChildren();
  if (!typed.startsWith("/")) {
    ui.commandHint.hidden = true;
    return;
  }
  const matches = COMMANDS.filter((command) => {
    const label = command.label.trim().toLowerCase();
    return label.startsWith(typed) || typed.startsWith(label);
  });
  ui.commandHint.hidden = matches.length === 0;
  for (const command of matches) {
    const button = document.createElement("button");
    button.type = "button";
    const name = document.createElement("strong");
    name.textContent = command.label.trim();
    const detail = document.createElement("span");
    detail.textContent = command.detail;
    button.append(name, detail);
    button.addEventListener("click", () => {
      ui.idea.value = command.label;
      ui.ideaCount.textContent = String([...ui.idea.value].length);
      renderCommandHint();
      ui.idea.focus();
      ui.idea.setSelectionRange(ui.idea.value.length, ui.idea.value.length);
    });
    ui.commandHint.append(button);
  }
}

function renderOutput() {
  const template = templateOf(state.templateId);
  const result = state.result;
  const output = ui.output.value;
  const missing = result?.missingProtected ?? [];
  ui.protectedActions.hidden = missing.length === 0;
  if (state.commandOutput) {
    ui.outputLabel.textContent = "Keyword command";
    ui.negativeCard.hidden = true;
    ui.outputCount.textContent = output ? `${[...output].length.toLocaleString()} characters` : "—";
    ui.outputCount.className = "count";
    ui.warnings.replaceChildren();
    return;
  }
  ui.negativeCard.hidden = !isWan(state.templateId);
  ui.outputLabel.textContent = isWan(state.templateId) ? "Positive prompt" : "Prompt";
  const chars = result && result.templateId === state.templateId ? result.charCount : [...output].length;
  const words = result && result.templateId === state.templateId ? result.wordCount : output.trim().split(/\s+/).filter(Boolean).length;
  const maxChars = result?.maxChars ?? template?.maxChars ?? null;
  const kind = result?.maxCharsKind ?? template?.maxCharsKind ?? "model";
  const maxWords = result?.maxWords ?? template?.maxWords ?? null;
  const parts = [countText(chars, maxChars, kind, "chars")];
  if (maxWords != null) parts.unshift(`${words} / ${maxWords} words`);
  ui.outputCount.textContent = output ? parts.join(" · ") : maxChars ? `0 / ${maxChars.toLocaleString()} ${kind === "suggested" ? "suggested" : "limit"}` : "—";
  ui.outputCount.className = output ? countClass(chars, maxChars) : "count";
  if (maxWords != null && words > maxWords) ui.outputCount.className = "count over";

  const negative = ui.negative.value;
  const negativeMax = result?.negativeMaxChars ?? template?.negativeMaxChars ?? 500;
  const negativeCount = result?.negativeCharCount ?? [...negative].length;
  ui.negativeCount.textContent = negative ? countText(negativeCount, negativeMax, "model", "chars") : `0 / ${negativeMax} limit`;
  ui.negativeCount.className = countClass(negativeCount, negativeMax);

  ui.warnings.replaceChildren();
  for (const warning of result?.warnings ?? []) {
    const item = document.createElement("li");
    item.textContent = warning;
    ui.warnings.append(item);
  }
}

function renderHistory(entries: HistoryEntry[]) {
  ui.historyEmpty.hidden = entries.length > 0;
  ui.historyList.replaceChildren();
  for (const entry of entries) {
    const card = document.createElement("article");
    card.className = "history-item";
    const restore = document.createElement("button");
    restore.type = "button";
    restore.className = "restore";
    const meta = document.createElement("div");
    meta.className = "history-meta";
    const when = document.createElement("span");
    when.textContent = new Date(entry.createdAt).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
    const pill = document.createElement("span");
    pill.className = "pill";
    pill.textContent = entry.templateTitle;
    meta.append(when, pill);
    if (entry.overLimit) {
      const over = document.createElement("span");
      over.className = "pill over";
      over.textContent = "Over limit";
      meta.append(over);
    }
    const idea = document.createElement("div");
    idea.className = "history-idea";
    idea.textContent = entry.idea;
    restore.append(meta, idea);
    restore.addEventListener("click", () => {
      ui.idea.value = entry.idea;
      ui.ideaCount.textContent = String([...entry.idea].length);
      if (entry.durationSecs) ui.duration.value = String(entry.durationSecs);
      applyTemplateId(entry.templateId);
      ui.output.value = entry.output;
      ui.negative.value = entry.negative ?? "";
      state.result = {
        templateId: entry.templateId,
        templateTitle: entry.templateTitle,
        output: entry.output,
        negative: entry.negative,
        charCount: [...entry.output].length,
        wordCount: entry.output.trim().split(/\s+/).filter(Boolean).length,
        negativeCharCount: entry.negative ? [...entry.negative].length : null,
        maxChars: templateOf(entry.templateId)?.maxChars ?? null,
        maxCharsKind: templateOf(entry.templateId)?.maxCharsKind ?? "model",
        maxWords: templateOf(entry.templateId)?.maxWords ?? null,
        negativeMaxChars: templateOf(entry.templateId)?.negativeMaxChars ?? null,
        outputKind: templateOf(entry.templateId)?.output ?? "text",
        overLimit: entry.overLimit,
        warnings: entry.warnings,
        historyId: entry.id,
        missingProtected: [],
      };
      state.commandOutput = false;
      renderOutput();
      showView("compose");
    });
    const actions = document.createElement("div");
    actions.className = "history-actions";
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "ghost";
    copy.textContent = "Copy";
    copy.addEventListener("click", () => {
      const text = entry.negative ? `${entry.output}\n\nNegative:\n${entry.negative}` : entry.output;
      void copyText(text);
    });
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "ghost";
    remove.textContent = "Delete";
    remove.addEventListener("click", () => {
      void api.deleteHistory(entry.id).then(renderHistory).catch((error: unknown) => showBanner(errorText(error)));
    });
    actions.append(copy, remove);
    card.append(restore, actions);
    ui.historyList.append(card);
  }
}

function fillModelList(list: HTMLDataListElement, models: ModelInfo[]) {
  list.replaceChildren();
  for (const model of models) {
    const option = document.createElement("option");
    option.value = model.id;
    option.label = model.free ? `${model.name} · free` : model.name;
    list.append(option);
  }
}

function fillTemplateSelects() {
  const current = ui.templateFile.value;
  ui.templateFile.replaceChildren();
  ui.defaultTemplate.replaceChildren();
  const templates = state.templates.length ? state.templates : FAMILIES.flatMap((family) =>
    family.options.map((option) => ({ id: option.id, title: option.label }) as Pick<TemplateSummary, "id" | "title">),
  );
  for (const template of templates) {
    const file = document.createElement("option");
    file.value = template.id;
    file.textContent = template.title;
    ui.templateFile.append(file);
    const fallback = document.createElement("option");
    fallback.value = template.id;
    fallback.textContent = template.title;
    ui.defaultTemplate.append(fallback);
  }
  if (current && [...ui.templateFile.options].some((option) => option.value === current)) {
    ui.templateFile.value = current;
  }
}

function applySettings(settings: SettingsView) {
  state.settings = settings;
  const backend = document.querySelectorAll<HTMLInputElement>('input[name="backend"]');
  for (const radio of backend) radio.checked = radio.value === settings.backend;
  ui.orModel.value = settings.openrouterModel;
  ui.ollamaUrl.value = settings.ollamaBaseUrl;
  ui.ollamaModel.value = settings.ollamaModel;
  ui.hotkey.value = settings.hotkey;
  ui.temperature.value = String(settings.temperature);
  ui.tempValue.textContent = settings.temperature.toFixed(1);
  if ([...ui.defaultTemplate.options].some((option) => option.value === settings.defaultTemplateId)) {
    ui.defaultTemplate.value = settings.defaultTemplateId;
  }
  ui.keyStatus.textContent = settings.keyConfigured
    ? settings.secretStore === "os-keychain"
      ? "A key is saved in the OS keychain. It is not written into settings."
      : "A key is saved in a private file because the OS keychain is unavailable. It is not written into settings."
    : settings.secretStore === "os-keychain"
      ? "No key saved. A new key goes into the OS keychain."
      : "No key saved. The OS keychain is unavailable, so a new key is stored in a private file.";
  ui.hotkeyStatus.textContent = settings.hotkeyError
    ? settings.hotkeyError
    : settings.hotkeyActive
      ? `${settings.hotkey} is active.`
      : "Save settings to register the hotkey.";
  ui.templatesPath.textContent = settings.templatesDir;
  const footer = document.getElementById("shortcut-footer");
  if (footer) footer.textContent = `Ctrl+Enter rewrites · Esc hides · ${settings.hotkey} toggles`;
}

function showView(name: "compose" | "history" | "settings") {
  for (const view of ["compose", "history", "settings"]) {
    const section = document.getElementById(`view-${view}`);
    if (section) section.hidden = view !== name;
    document.querySelector<HTMLButtonElement>(`[data-view="${view}"]`)?.classList.toggle("active", view === name);
  }
  if (name === "history") {
    void api.listHistory().then(renderHistory).catch((error: unknown) => showBanner(errorText(error)));
  }
  if (name === "settings") {
    void loadKeywords();
  }
}

async function loadKeywords() {
  try {
    const file = await api.getProtectedKeywords();
    ui.keywords.value = file.text;
    ui.keywordsPath.textContent = file.path;
  } catch (error) {
    showBanner(errorText(error));
  }
}

async function runKeywordCommand(line: string) {
  state.busy = true;
  ui.rewrite.disabled = true;
  ui.rewrite.textContent = "Running…";
  try {
    const result = await api.runKeywordCommand(line);
    state.commandOutput = true;
    state.result = null;
    ui.output.value = result.message;
    ui.negative.value = "";
    ui.keywords.value = result.text;
    renderOutput();
  } catch (error) {
    showBanner(errorText(error));
  } finally {
    state.busy = false;
    ui.rewrite.disabled = false;
    ui.rewrite.textContent = "Rewrite";
  }
}

function appendMissingTerms() {
  const missing = state.result?.missingProtected ?? [];
  if (!missing.length) return;
  const current = ui.output.value.replace(/\s+$/, "");
  const addition = missing.join(", ");
  const output = current ? `${current}\n${addition}` : addition;
  ui.output.value = output;
  if (state.result) {
    state.result = {
      ...state.result,
      output,
      charCount: [...output].length,
      wordCount: output.trim().split(/\s+/).filter(Boolean).length,
      missingProtected: [],
      warnings: state.result.warnings.filter((warning) => !warning.startsWith("Protected terms missing")),
    };
  }
  renderOutput();
}

async function loadTemplateEditor() {
  const id = ui.templateFile.value;
  if (!id) return;
  try {
    ui.templateEditor.value = await api.readTemplate(id);
  } catch (error) {
    showBanner(errorText(error));
  }
}

async function refreshModels(which: "openrouter" | "ollama") {
  clearBanner();
  try {
    if (which === "openrouter") {
      const models = await api.listModels("openrouter", ui.ollamaUrl.value, ui.apiKey.value);
      fillModelList(ui.orModels, models);
      ui.orStatus.textContent = models.length ? `${models.length} models. Free models are marked. You can still type any id.` : "No models came back.";
    } else {
      const models = await api.listModels("ollama", ui.ollamaUrl.value, "");
      fillModelList(ui.ollamaModels, models);
      ui.ollamaStatus.textContent = models.length
        ? `${models.length} local models.`
        : "Ollama answered, but no models are pulled yet.";
    }
  } catch (error) {
    const message = errorText(error);
    if (which === "openrouter") ui.orStatus.textContent = message;
    else ui.ollamaStatus.textContent = message;
    showBanner(message);
  }
}

async function rewrite() {
  if (state.busy) return;
  clearBanner();
  const idea = ui.idea.value.trim();
  if (!idea) {
    showBanner("Write an idea before rewriting.");
    ui.idea.focus();
    return;
  }
  if (idea.startsWith("/")) {
    await runKeywordCommand(idea);
    return;
  }
  const bounds = durationBounds(state.templateId);
  const duration = bounds ? Number(ui.duration.value) : null;
  if (bounds && duration != null && (!Number.isInteger(duration) || duration < bounds.min || duration > bounds.max)) {
    showBanner(`Duration has to be a whole number from ${bounds.min} to ${bounds.max} seconds.`);
    return;
  }
  state.busy = true;
  ui.rewrite.disabled = true;
  ui.rewrite.textContent = "Rewriting…";
  document.getElementById("view-compose")?.setAttribute("aria-busy", "true");
  try {
    const result = await api.rewrite(idea, state.templateId, duration);
    state.commandOutput = false;
    state.result = result;
    ui.output.value = result.output;
    ui.negative.value = result.negative ?? "";
    renderOutput();
  } catch (error) {
    showBanner(errorText(error));
  } finally {
    state.busy = false;
    ui.rewrite.disabled = false;
    ui.rewrite.textContent = "Rewrite";
    document.getElementById("view-compose")?.removeAttribute("aria-busy");
  }
}

function stopRecording() {
  if (state.recording) {
    window.removeEventListener("keydown", state.recording, true);
    state.recording = false;
    ui.record.textContent = "Record";
  }
}

function startRecording() {
  stopRecording();
  ui.record.textContent = "Press keys";
  const handler = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      stopRecording();
      return;
    }
    if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.metaKey) parts.push("Super");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    const key = namedKey(event);
    if (!key || parts.length === 0) {
      showBanner("Hold Ctrl, Alt, or Shift, then press the main key.");
      return;
    }
    parts.push(key);
    ui.hotkey.value = parts.join("+");
    stopRecording();
  };
  state.recording = handler;
  window.addEventListener("keydown", handler, true);
}

function namedKey(event: KeyboardEvent): string | null {
  if (event.code === "Space") return "Space";
  if (event.code.startsWith("Key") && event.code.length === 4) return event.code.slice(3);
  if (event.code.startsWith("Digit")) return event.code.slice(5);
  if (/^F\d{1,2}$/.test(event.code)) return event.code;
  const named: Record<string, string> = {
    Enter: "Enter",
    Tab: "Tab",
    Minus: "-",
    Equal: "=",
    Backquote: "`",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    Comma: ",",
    Period: ".",
    Slash: "/",
  };
  return named[event.code] ?? null;
}

function selectedBackend(): "openrouter" | "ollama" {
  const checked = document.querySelector<HTMLInputElement>('input[name="backend"]:checked');
  return checked?.value === "ollama" ? "ollama" : "openrouter";
}

async function boot() {
  if (!inTauri()) ui.note.hidden = false;
  fillFamilies();
  ui.duration.value = "6";
  try {
    const catalog = await api.listTemplates();
    state.templates = catalog.templates;
    if (catalog.invalid.length) {
      showBanner(catalog.invalid.map((item) => `${item.file}: ${item.error}`).join(" "));
    }
  } catch (error) {
    showBanner(errorText(error));
  }
  fillTemplateSelects();
  try {
    const settings = await api.getSettings();
    applySettings(settings);
    applyTemplateId(settings.defaultTemplateId || "wan22-t2v");
  } catch (error) {
    applyTemplateId("wan22-t2v");
    showBanner(errorText(error));
  }
  renderOutput();
  void loadKeywords();
  const footer = document.createElement("p");
  footer.id = "shortcut-footer";
  footer.className = "hint";
  footer.style.marginTop = "12px";
  footer.textContent = `Ctrl+Enter rewrites · Esc hides · ${state.settings?.hotkey ?? "Ctrl+Shift+Space"} toggles`;
  document.getElementById("view-compose")?.append(footer);
  void api.onOpenSettings(() => showView("settings"));
}

document.querySelectorAll<HTMLButtonElement>("[data-view]").forEach((button) => {
  button.addEventListener("click", () => {
    const view = button.dataset.view;
    if (view === "compose" || view === "history" || view === "settings") showView(view);
  });
});

ui.family.addEventListener("change", () => {
  const family = FAMILIES.find((item) => item.id === ui.family.value);
  if (!family) return;
  state.templateId = family.options[0].id;
  state.result = null;
  state.commandOutput = false;
  ui.output.value = "";
  ui.negative.value = "";
  renderVariants();
  renderOutput();
});

ui.idea.addEventListener("input", () => {
  ui.ideaCount.textContent = String([...ui.idea.value].length);
  renderCommandHint();
});

ui.rewrite.addEventListener("click", () => void rewrite());
ui.appendProtected.addEventListener("click", () => appendMissingTerms());
document.getElementById("save-keywords")?.addEventListener("click", () => {
  void api.saveProtectedKeywords(ui.keywords.value).then((file) => {
    ui.keywords.value = file.text;
    ui.keywordsPath.textContent = file.path;
    flash("Protected keywords saved.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});
ui.copyOutput.addEventListener("click", () => void copyText(ui.output.value));
ui.copyNegative.addEventListener("click", () => void copyText(ui.negative.value));
ui.copyBoth.addEventListener("click", () => {
  const text = ui.negative.value.trim()
    ? `${ui.output.value}\n\nNegative:\n${ui.negative.value}`
    : ui.output.value;
  void copyText(text);
});

document.getElementById("hide-btn")?.addEventListener("click", () => {
  if (!inTauri()) flash("Esc hides the overlay in the desktop app.");
  else void api.hide();
});

ui.clearHistory.addEventListener("click", () => {
  if (!confirm("Clear the saved rewrites on this machine?")) return;
  void api.clearHistory().then(() => renderHistory([])).catch((error: unknown) => showBanner(errorText(error)));
});

document.getElementById("save-key")?.addEventListener("click", () => {
  void api.setKey(ui.apiKey.value).then((settings) => {
    ui.apiKey.value = "";
    applySettings(settings);
    flash("Key saved.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});

document.getElementById("clear-key")?.addEventListener("click", () => {
  void api.clearKey().then((settings) => {
    ui.apiKey.value = "";
    applySettings(settings);
    flash("Key removed.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});

document.getElementById("or-refresh")?.addEventListener("click", () => void refreshModels("openrouter"));
document.getElementById("ollama-refresh")?.addEventListener("click", () => void refreshModels("ollama"));
ui.record.addEventListener("click", () => {
  if (state.recording) stopRecording();
  else startRecording();
});

ui.temperature.addEventListener("input", () => {
  ui.tempValue.textContent = Number(ui.temperature.value).toFixed(1);
});

document.getElementById("save-settings")?.addEventListener("click", () => {
  clearBanner();
  void api.saveSettings({
    backend: selectedBackend(),
    openrouterModel: ui.orModel.value.trim(),
    ollamaBaseUrl: ui.ollamaUrl.value.trim(),
    ollamaModel: ui.ollamaModel.value.trim(),
    hotkey: ui.hotkey.value.trim(),
    defaultTemplateId: ui.defaultTemplate.value,
    temperature: Number(ui.temperature.value),
  }).then((settings) => {
    applySettings(settings);
    flash("Settings saved.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});

document.getElementById("open-templates")?.addEventListener("click", () => {
  void api.openTemplates().catch((error: unknown) => showBanner(errorText(error)));
});

ui.templateFile.addEventListener("change", () => void loadTemplateEditor());
document.getElementById("save-template")?.addEventListener("click", () => {
  void api.saveTemplate(ui.templateFile.value, ui.templateEditor.value).then(async (catalog) => {
    state.templates = catalog.templates;
    fillTemplateSelects();
    renderOutput();
    flash("Template saved. The next rewrite uses it.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});
document.getElementById("reset-template")?.addEventListener("click", () => {
  const id = ui.templateFile.value;
  if (!confirm(`Reset ${id} to the bundled default?`)) return;
  void api.resetTemplate(id).then(async (catalog) => {
    state.templates = catalog.templates;
    fillTemplateSelects();
    ui.templateFile.value = id;
    await loadTemplateEditor();
    flash("Template reset.");
  }).catch((error: unknown) => showBanner(errorText(error)));
});

window.addEventListener("keydown", (event) => {
  if (state.recording) return;
  if (event.key === "Escape") {
    event.preventDefault();
    if (!inTauri()) flash("Esc hides the overlay in the desktop app.");
    else void api.hide();
    return;
  }
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
    const compose = document.getElementById("view-compose");
    if (compose && !compose.hidden) {
      event.preventDefault();
      void rewrite();
    }
  }
});

void boot().then(() => void loadTemplateEditor());
