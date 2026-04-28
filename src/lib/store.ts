import { create } from "zustand";
import type { ApiKeyStatus, DictationConfig, ProviderKey } from "@/lib/tauri";
import { tauri } from "@/lib/tauri";

interface AppStore {
  config: DictationConfig | null;
  keys: Record<ProviderKey, ApiKeyStatus>;
  loading: boolean;

  load: () => Promise<void>;
  setConfig: (patch: Partial<DictationConfig>) => Promise<void>;
  setKey: (provider: ProviderKey, key: string) => Promise<void>;
  removeKey: (provider: ProviderKey) => Promise<void>;
  validateKey: (provider: ProviderKey) => Promise<{ ok: boolean; detail?: string }>;
}

const emptyKey = (provider: ProviderKey): ApiKeyStatus => ({
  provider,
  hasKey: false,
  validatedAt: null,
});

const emptyKeys: Record<ProviderKey, ApiKeyStatus> = {
  anthropic: emptyKey("anthropic"),
  openrouter: emptyKey("openrouter"),
  openai: emptyKey("openai"),
  deepgram: emptyKey("deepgram"),
  groq: emptyKey("groq"),
  elevenlabs: emptyKey("elevenlabs"),
};

export const useApp = create<AppStore>((set, get) => ({
  config: null,
  keys: emptyKeys,
  loading: false,

  async load() {
    set({ loading: true });
    const [config, statuses] = await Promise.all([
      tauri.getConfig(),
      tauri.listApiKeyStatuses(),
    ]);
    const keys = { ...emptyKeys };
    for (const s of statuses) keys[s.provider] = s;
    set({ config, keys, loading: false });
  },

  async setConfig(patch) {
    const config = await tauri.updateConfig(patch);
    set({ config });
  },

  async setKey(provider, key) {
    const status = await tauri.setApiKey(provider, key);
    set({ keys: { ...get().keys, [provider]: status } });
  },

  async removeKey(provider) {
    await tauri.deleteApiKey(provider);
    set({ keys: { ...get().keys, [provider]: emptyKey(provider) } });
  },

  async validateKey(provider) {
    const result = await tauri.validateApiKey(provider);
    if (result.ok) {
      const status = await tauri.getApiKeyStatus(provider);
      set({ keys: { ...get().keys, [provider]: status } });
    }
    return result;
  },
}));
