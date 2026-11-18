import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Backend = "openrouter" | "ollama";
export type LimitKind = "model" | "suggested";
export type OutputKind = "text" | "wan_pair";

export interface SettingsView {
  backend: Backend;
  openrouterModel: string;
  ollamaBaseUrl: string;
  ollamaModel: string;
  hotkey: string;
  defaultTemplateId: string;
  temperature: number;
  keyConfigured: boolean;
  secretStore: "os-keychain" | "private-file";
  templatesDir: string;
  configDir: string;
  hotkeyActive: boolean;
  hotkeyError: string | null;
}

export interface SettingsInput {
  backend: Backend;
  openrouterModel: string;
  ollamaBaseUrl: string;
  ollamaModel: string;
  hotkey: string;
  defaultTemplateId: string;
  temperature: number;
}

export interface TemplateSummary {
  id: string;
  family: string;
  variant: string;
  title: string;
  maxChars: number | null;
  maxCharsKind: LimitKind;
  maxWords: number | null;
  negativeMaxChars: number | null;
  output: OutputKind;
}

export interface TemplateCatalog {
  templates: TemplateSummary[];
  invalid: { file: string; error: string }[];
}

export interface ModelInfo {
  id: string;
  name: string;
  free: boolean;
}

export interface RewriteResult {
  templateId: string;
  templateTitle: string;
  output: string;
  negative: string | null;
  charCount: number;
  wordCount: number;
  negativeCharCount: number | null;
  maxChars: number | null;
  maxCharsKind: LimitKind;
  maxWords: number | null;
  negativeMaxChars: number | null;
  outputKind: OutputKind;
  overLimit: boolean;
  warnings: string[];
  historyId: string;
}

export interface HistoryEntry {
  id: string;
  createdAt: string;
  idea: string;
  templateId: string;
  templateTitle: string;
  durationSecs: number | null;
  output: string;
  negative: string | null;
  backend: string;
  model: string;
  overLimit: boolean;
  warnings: string[];
}

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function errorText(error: unknown): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message.trim()) return error.message;
  return "Something went wrong.";
}

const STORAGE = "prompt-rewriter-preview";

function previewSettings(): SettingsView {
  const saved = localStorage.getItem(STORAGE);
  const base: SettingsView = {
    backend: "openrouter",
    openrouterModel: "",
    ollamaBaseUrl: "http://localhost:11434",
    ollamaModel: "",
    hotkey: "Ctrl+Shift+Space",
    defaultTemplateId: "wan22-t2v",
    temperature: 0.4,
    keyConfigured: false,
    secretStore: "os-keychain",
    templatesDir: "Shown in the desktop app",
    configDir: "Shown in the desktop app",
    hotkeyActive: false,
    hotkeyError: null,
  };
  if (!saved) return base;
  try {
    return { ...base, ...JSON.parse(saved) };
  } catch {
    return base;
  }
}

function storePreview(settings: SettingsView) {
  localStorage.setItem(STORAGE, JSON.stringify(settings));
}

const PREVIEW_TEMPLATES: TemplateSummary[] = [
  { id: "wan22-t2v", family: "wan22", variant: "t2v", title: "Wan 2.2 · Text to video", maxChars: 800, maxCharsKind: "model", maxWords: null, negativeMaxChars: 500, output: "wan_pair" },
  { id: "wan22-i2v", family: "wan22", variant: "i2v", title: "Wan 2.2 · Image to video", maxChars: 800, maxCharsKind: "model", maxWords: 100, negativeMaxChars: 500, output: "wan_pair" },
  { id: "minimax-h3", family: "minimax-h3", variant: "t2va", title: "MiniMax H3", maxChars: 7000, maxCharsKind: "model", maxWords: null, negativeMaxChars: null, output: "text" },
  { id: "seedance-1.0", family: "seedance", variant: "1.0", title: "Seedance 1.0", maxChars: 2000, maxCharsKind: "suggested", maxWords: null, negativeMaxChars: null, output: "text" },
  { id: "seedance-2.0", family: "seedance", variant: "2.0", title: "Seedance 2.0", maxChars: 2500, maxCharsKind: "suggested", maxWords: null, negativeMaxChars: null, output: "text" },
  { id: "seedance-2.5", family: "seedance", variant: "2.5", title: "Seedance 2.5", maxChars: 4000, maxCharsKind: "suggested", maxWords: null, negativeMaxChars: null, output: "text" },
  { id: "seedream-generate", family: "seedream", variant: "generate", title: "Seedream · Generate", maxChars: 800, maxCharsKind: "suggested", maxWords: null, negativeMaxChars: null, output: "text" },
  { id: "seedream-edit", family: "seedream", variant: "edit", title: "Seedream · Edit", maxChars: 600, maxCharsKind: "suggested", maxWords: null, negativeMaxChars: null, output: "text" },
];

let previewHistory: HistoryEntry[] = [];

function previewRewrite(idea: string, templateId: string, duration: number | null): RewriteResult {
  const template = PREVIEW_TEMPLATES.find((item) => item.id === templateId) ?? PREVIEW_TEMPLATES[0];
  const overflow = idea.toUpperCase().includes("OVERFLOW");
  let output = `Preview rewrite for ${template.title}. ${idea.trim()}`;
  let negative: string | null = null;
  if (template.output === "wan_pair") {
    output = overflow ? `${"daylight, medium shot. ".repeat(40)}${idea}` : `Daylight, medium shot. ${idea.trim()} The camera pushes in slowly.`;
    negative = "blur, watermark, subtitles";
    if (templateId === "wan22-i2v" && !/camera/i.test(idea)) {
      output = `fixed camera. ${idea.trim()}`;
    }
  } else if (templateId === "minimax-h3") {
    output = `integrated_multimodal_description: [Shot 1] Live-action, cinematic. ${idea.trim()} The camera holds a static shot.\n\noverall_soundscape: Room tone and a single footstep.\n\nnon_diegetic_music: N/A`;
  } else if (templateId === "seedance-2.0") {
    output = `Shot 1: locked-off camera. ${idea.trim()}\nkeep it subtitle-free.`;
  } else if (templateId === "seedance-2.5") {
    const end = duration ?? 5;
    output = `Asset map:\nNo reference assets.\n\nSummary:\n${idea.trim()}\n\nShots:\n0-${end}s: locked-off camera. ${idea.trim()}\n\nConstants:\nNo subtitles.\nNo BGM; only ambient and action sounds.`;
  } else if (templateId === "seedance-1.0") {
    output = `${idea.trim()}\n--duration ${duration ?? 5}`;
  } else if (templateId === "seedream-edit") {
    output = `${idea.trim()} Keep the face, pose, and framing unchanged.`;
  }
  const charCount = [...output].length;
  const wordCount = output.trim().split(/\s+/).filter(Boolean).length;
  const negativeCharCount = negative ? [...negative].length : null;
  const warnings: string[] = [];
  let overLimit = false;
  if (template.maxChars != null && charCount > template.maxChars) {
    overLimit = true;
    warnings.push(`The prompt is ${charCount} characters, over the ${template.maxChars} character ${template.maxCharsKind === "model" ? "model limit" : "suggested length"}.`);
  }
  if (template.maxWords != null && wordCount > template.maxWords) {
    overLimit = true;
    warnings.push(`The prompt is ${wordCount} words, over the ${template.maxWords} word model limit.`);
  }
  const entry: HistoryEntry = {
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    idea,
    templateId,
    templateTitle: template.title,
    durationSecs: duration,
    output,
    negative,
    backend: "preview",
    model: "preview",
    overLimit,
    warnings,
  };
  previewHistory = [entry, ...previewHistory].slice(0, 50);
  return {
    templateId,
    templateTitle: template.title,
    output,
    negative,
    charCount,
    wordCount,
    negativeCharCount,
    maxChars: template.maxChars,
    maxCharsKind: template.maxCharsKind,
    maxWords: template.maxWords,
    negativeMaxChars: template.negativeMaxChars,
    outputKind: template.output,
    overLimit,
    warnings,
    historyId: entry.id,
  };
}

export const api = {
  async getSettings(): Promise<SettingsView> {
    if (!inTauri()) return previewSettings();
    return invoke<SettingsView>("get_settings");
  },
  async saveSettings(input: SettingsInput): Promise<SettingsView> {
    if (!inTauri()) {
      const next = { ...previewSettings(), ...input, hotkeyActive: false, hotkeyError: null };
      storePreview(next);
      return next;
    }
    return invoke<SettingsView>("save_settings", { input });
  },
  async setKey(apiKey: string): Promise<SettingsView> {
    if (!inTauri()) {
      const next = { ...previewSettings(), keyConfigured: true };
      storePreview(next);
      return next;
    }
    return invoke<SettingsView>("set_openrouter_key", { input: { apiKey } });
  },
  async clearKey(): Promise<SettingsView> {
    if (!inTauri()) {
      const next = { ...previewSettings(), keyConfigured: false };
      storePreview(next);
      return next;
    }
    return invoke<SettingsView>("clear_openrouter_key");
  },
  async listModels(backend: Backend, ollamaBaseUrl: string, apiKey: string): Promise<ModelInfo[]> {
    if (!inTauri()) {
      if (backend === "ollama") {
        return [
          { id: "llama3.2:latest", name: "llama3.2:latest", free: true },
          { id: "qwen2.5:7b", name: "qwen2.5:7b", free: true },
        ];
      }
      return [
        { id: "google/gemma-2-9b-it:free", name: "Gemma 2 9B (free)", free: true },
        { id: "anthropic/claude-sonnet-4.5", name: "Claude Sonnet 4.5", free: false },
      ];
    }
    const result = await invoke<{ models: ModelInfo[] }>("list_backend_models", {
      query: { backend, ollamaBaseUrl, apiKey },
    });
    return result.models;
  },
  async listTemplates(): Promise<TemplateCatalog> {
    if (!inTauri()) return { templates: PREVIEW_TEMPLATES, invalid: [] };
    return invoke<TemplateCatalog>("list_templates");
  },
  async readTemplate(id: string): Promise<string> {
    if (!inTauri()) return `---\nid: ${id}\n---\n\nTemplate editing runs against the files in the desktop app.`;
    const doc = await invoke<{ id: string; contents: string }>("read_template", { input: { id } });
    return doc.contents;
  },
  async saveTemplate(id: string, contents: string): Promise<TemplateCatalog> {
    if (!inTauri()) return { templates: PREVIEW_TEMPLATES, invalid: [] };
    return invoke<TemplateCatalog>("save_template", { input: { id, contents } });
  },
  async resetTemplate(id: string): Promise<TemplateCatalog> {
    if (!inTauri()) return { templates: PREVIEW_TEMPLATES, invalid: [] };
    return invoke<TemplateCatalog>("reset_template", { input: { id } });
  },
  async openTemplates(): Promise<void> {
    if (!inTauri()) return;
    await invoke("open_templates_dir");
  },
  async hide(): Promise<void> {
    if (!inTauri()) return;
    await invoke("hide_window");
  },
  async rewrite(idea: string, templateId: string, durationSecs: number | null): Promise<RewriteResult> {
    if (!inTauri()) {
      await new Promise((resolve) => setTimeout(resolve, 450));
      if (idea.toLowerCase().includes("fail-key")) throw new Error("OpenRouter rejected the API key.");
      if (idea.toLowerCase().includes("fail-ollama")) {
        throw new Error("Ollama is not running at http://localhost:11434. Start it with `ollama serve`, or change the base URL in Settings.");
      }
      return previewRewrite(idea, templateId, durationSecs);
    }
    return invoke<RewriteResult>("rewrite", { input: { idea, templateId, durationSecs } });
  },
  async listHistory(): Promise<HistoryEntry[]> {
    if (!inTauri()) return previewHistory;
    return invoke<HistoryEntry[]>("list_history");
  },
  async deleteHistory(id: string): Promise<HistoryEntry[]> {
    if (!inTauri()) {
      previewHistory = previewHistory.filter((entry) => entry.id !== id);
      return previewHistory;
    }
    return invoke<HistoryEntry[]>("delete_history_entry", { input: { id } });
  },
  async clearHistory(): Promise<void> {
    if (!inTauri()) {
      previewHistory = [];
      return;
    }
    await invoke("clear_history");
  },
  onOpenSettings(handler: () => void): Promise<UnlistenFn> | null {
    if (!inTauri()) return null;
    return listen("open-settings", () => handler());
  },
};
