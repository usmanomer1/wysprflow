import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Strongly-typed bridge to the Rust commands defined in
 * src-tauri/src/commands.rs. Keep in sync with the Rust side.
 */

export type ProviderKey =
  | "anthropic"
  | "openrouter"
  | "openai"
  | "deepgram"
  | "groq"
  | "elevenlabs";

export interface ApiKeyStatus {
  provider: ProviderKey;
  hasKey: boolean;
  validatedAt: string | null;
}

export interface DictationConfig {
  sttProvider: "deepgram" | "groq" | "openai" | "elevenlabs" | "local";
  llmProvider: "anthropic" | "openrouter" | "openai" | "off";
  llmModel: string;
  hotkey: string;
  hotkeyMode: "hold" | "toggle" | "tap-toggle";
  language: string;
  autoCleanup: "none" | "light" | "medium" | "high";
  microphoneDevice: string;
  preserveClipboard: boolean;
  playSounds: boolean;
  ideFileTagging: boolean;
  translateTo: string;
  snippetsEnabled: boolean;
  customCleanupPrompt: string;
  launchAtLogin: boolean;
  setupCompleted: boolean;
}

export interface AudioInputDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface TranscriptChunk {
  text: string;
  isFinal: boolean;
  confidence: number;
}

export interface HudStateEvent {
  state: "idle" | "initializing" | "listening" | "processing" | "error";
  message?: string;
  level?: number;
}

export type PermissionState = "granted" | "denied" | "notDetermined";

export interface PermissionStatus {
  microphone: PermissionState;
  accessibility: PermissionState;
  inputMonitoring: PermissionState;
}

export interface InstallationStatus {
  bundlePath: string;
  inApplications: boolean;
  isTranslocated: boolean;
}

export interface DictEntry {
  id: number;
  word: string;
  isStarred: boolean;
  autoLearned: boolean;
  usageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface Snippet {
  id: number;
  trigger: string;
  expansion: string;
  createdAt: string;
  updatedAt: string;
}

export interface HistoryEntry {
  id: string;
  timestamp: string;
  rawTranscript: string;
  cleanedTranscript: string;
  sourceApp: string | null;
  durationMs: number | null;
  wordCount: number;
  error: string | null;
}

export const tauri = {
  // Settings
  async getConfig(): Promise<DictationConfig> {
    return invoke<DictationConfig>("get_config");
  },
  async updateConfig(patch: Partial<DictationConfig>): Promise<DictationConfig> {
    return invoke<DictationConfig>("update_config", { patch });
  },

  // API keys (stored in macOS Keychain on the Rust side)
  async getApiKeyStatus(provider: ProviderKey): Promise<ApiKeyStatus> {
    return invoke<ApiKeyStatus>("get_api_key_status", { provider });
  },
  async listApiKeyStatuses(): Promise<ApiKeyStatus[]> {
    return invoke<ApiKeyStatus[]>("list_api_key_statuses");
  },
  async setApiKey(provider: ProviderKey, key: string): Promise<ApiKeyStatus> {
    return invoke<ApiKeyStatus>("set_api_key", { provider, key });
  },
  async deleteApiKey(provider: ProviderKey): Promise<void> {
    return invoke("delete_api_key", { provider });
  },
  async validateApiKey(provider: ProviderKey): Promise<{ ok: boolean; detail?: string }> {
    return invoke("validate_api_key", { provider });
  },

  // Hotkey
  async setHotkey(accelerator: string): Promise<void> {
    return invoke("set_hotkey", { accelerator });
  },
  async listAudioInputDevices(): Promise<AudioInputDevice[]> {
    return invoke<AudioInputDevice[]>("list_audio_input_devices");
  },

  // Permissions (macOS)
  async checkPermissions(): Promise<PermissionStatus> {
    return invoke("check_permissions");
  },
  async getInstallationStatus(): Promise<InstallationStatus> {
    return invoke("get_installation_status");
  },
  async moveToApplications(): Promise<void> {
    return invoke("move_to_applications");
  },
  async requestMicrophone(): Promise<boolean> {
    return invoke("request_microphone");
  },
  async openAccessibilitySettings(): Promise<void> {
    return invoke("open_accessibility_settings");
  },
  async requestAccessibility(): Promise<PermissionStatus> {
    return invoke("request_accessibility");
  },
  async openInputMonitoringSettings(): Promise<void> {
    return invoke("open_input_monitoring_settings");
  },
  async openMicrophoneSettings(): Promise<void> {
    return invoke("open_microphone_settings");
  },

  // Dictation control
  async startDictation(): Promise<void> {
    return invoke("start_dictation");
  },
  async stopDictation(): Promise<void> {
    return invoke("stop_dictation");
  },

  // Dictionary
  async listDictionary(): Promise<DictEntry[]> {
    return invoke<DictEntry[]>("list_dictionary");
  },
  async addDictionaryWord(word: string): Promise<DictEntry> {
    return invoke<DictEntry>("add_dictionary_word", { word });
  },
  async deleteDictionaryWord(id: number): Promise<void> {
    return invoke("delete_dictionary_word", { id });
  },
  async toggleDictionaryStar(id: number): Promise<DictEntry> {
    return invoke<DictEntry>("toggle_dictionary_star", { id });
  },

  // Snippets
  async listSnippets(): Promise<Snippet[]> {
    return invoke<Snippet[]>("list_snippets");
  },
  async upsertSnippet(
    id: number | null,
    trigger: string,
    expansion: string,
  ): Promise<Snippet> {
    return invoke<Snippet>("upsert_snippet", { id, trigger, expansion });
  },
  async deleteSnippet(id: number): Promise<void> {
    return invoke("delete_snippet", { id });
  },

  // History
  async listHistory(limit = 200): Promise<HistoryEntry[]> {
    return invoke<HistoryEntry[]>("list_history", { limit });
  },
  async searchHistory(query: string, limit = 200): Promise<HistoryEntry[]> {
    return invoke<HistoryEntry[]>("search_history", { query, limit });
  },
  async deleteHistoryEntry(id: string): Promise<void> {
    return invoke("delete_history_entry", { id });
  },
  async clearHistory(): Promise<void> {
    return invoke("clear_history");
  },
};

export const events = {
  async onTranscript(cb: (e: TranscriptChunk) => void): Promise<UnlistenFn> {
    return listen<TranscriptChunk>("transcript", (e) => cb(e.payload));
  },
  async onTranscriptCleaned(cb: (text: string) => void): Promise<UnlistenFn> {
    return listen<string>("transcript-cleaned", (e) => cb(e.payload));
  },
  async onHudState(cb: (e: HudStateEvent) => void): Promise<UnlistenFn> {
    return listen<HudStateEvent>("hud-state", (e) => cb(e.payload));
  },
};
