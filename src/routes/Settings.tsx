import { useEffect, useState } from "react";
import {
  Mic,
  Keyboard,
  BookOpen,
  History,
  Info,
  Sparkles,
  Globe,
  LockKeyhole,
  Power,
  KeyRound,
  Clipboard,
  Sliders,
  Github,
  Pencil,
} from "lucide-react";

import { useApp } from "@/lib/store";
import { ApiKeysSection } from "@/components/ApiKeysSection";
import { HotkeySection } from "@/components/HotkeySection";
import { DictionarySection } from "@/components/DictionarySection";
import { SnippetsSection } from "@/components/SnippetsSection";
import { HistorySection } from "@/components/HistorySection";
import { SettingsCard } from "@/components/SettingsCard";
import {
  BehaviorSettingsSection,
  PermissionsSection,
  PromptSettingsSection,
  ProviderRoutingSection,
  StartupSettingsSection,
  TranslationSettingsSection,
  VoiceSettingsSection,
} from "@/components/GeneralSettingsSections";
import Setup from "@/routes/Setup";
import { cn } from "@/lib/utils";
import logoMark from "@/assets/wysprflow-logo.png";

type SettingsTab = "general" | "dictionary" | "snippets" | "history" | "prompts" | "about";

const tabs: ReadonlyArray<{
  id: SettingsTab;
  title: string;
  icon: React.ComponentType<{ className?: string }>;
}> = [
  { id: "general", title: "General", icon: Sliders },
  { id: "dictionary", title: "Dictionary", icon: BookOpen },
  { id: "snippets", title: "Snippets", icon: Sparkles },
  { id: "history", title: "Run Log", icon: History },
  { id: "prompts", title: "Prompts", icon: Pencil },
  { id: "about", title: "About", icon: Info },
];

export default function Settings() {
  const load = useApp((s) => s.load);
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");

  useEffect(() => {
    load().catch(() => {
      /* backend may not be ready during cold start */
    });
  }, [load]);

  // Gate on first-run setup. Once user clicks "Get started" we flip the flag and
  // load() refreshes the store, dropping us into the regular settings view.
  if (config && !config.setupCompleted) {
    return (
      <Setup
        onComplete={async () => {
          await setConfig({ setupCompleted: true });
        }}
      />
    );
  }

  return (
    <div className="flex h-full">
      <aside
        data-tauri-drag-region
        className="flex w-52 shrink-0 flex-col gap-2 border-r border-border bg-muted/30 px-3 pb-3 pt-14"
      >
        <div className="rounded-xl border border-border/60 bg-background/80 px-3 py-3 shadow-sm">
          <div className="flex items-center gap-2.5">
            <img
              src={logoMark}
              alt="wysprflow"
              className="size-9 rounded-[10px] border border-border/60 shadow-sm"
            />
            <div>
              <h1 className="text-sm font-semibold tracking-tight">wysprflow</h1>
              <p className="text-[10px] text-muted-foreground">v0.1.0</p>
            </div>
          </div>
        </div>
        <nav className="space-y-0.5">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const active = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={cn(
                  "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm transition-colors",
                  active
                    ? "bg-foreground/10 text-foreground"
                    : "text-muted-foreground hover:bg-foreground/5 hover:text-foreground",
                )}
              >
                <Icon className="size-3.5 shrink-0" />
                <span>{tab.title}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-2xl space-y-3 px-6 pb-6 pt-8">
          {activeTab === "general" && <GeneralTab />}
          {activeTab === "dictionary" && <DictionaryTab />}
          {activeTab === "snippets" && <SnippetsTab />}
          {activeTab === "history" && <HistoryTab />}
          {activeTab === "prompts" && <PromptsTab />}
          {activeTab === "about" && <AboutTab />}
        </div>
      </main>
    </div>
  );
}

function GeneralTab() {
  return (
    <>
      <SettingsCard
        title="Providers"
        icon={Sliders}
        description="Choose the active cleanup path and how aggressive post-processing should be."
      >
        <ProviderRoutingSection />
      </SettingsCard>

      <SettingsCard
        title="API keys"
        icon={KeyRound}
        description="Bring your own keys. Stored locally in development and in Keychain for packaged macOS builds."
      >
        <ApiKeysSection />
      </SettingsCard>

      <SettingsCard title="Hotkey" icon={Keyboard}>
        <HotkeySection />
      </SettingsCard>

      <SettingsCard
        title="Voice"
        icon={Mic}
        description="Pick the microphone and pin the recognition language when auto-detect is noisy."
      >
        <VoiceSettingsSection />
      </SettingsCard>

      <SettingsCard
        title="Permissions"
        icon={LockKeyhole}
        description="Live status for the macOS permissions dictation depends on."
      >
        <PermissionsSection />
      </SettingsCard>

      <SettingsCard
        title="Behavior"
        icon={Clipboard}
        description="Clipboard handling, snippet expansion, and audible feedback."
      >
        <BehaviorSettingsSection />
      </SettingsCard>

      <SettingsCard
        title="Translation"
        icon={Globe}
        description="Translate the final pasted text after transcription and cleanup."
      >
        <TranslationSettingsSection />
      </SettingsCard>

      <SettingsCard
        title="Startup"
        icon={Power}
        description="Launch the app automatically on login."
      >
        <StartupSettingsSection />
      </SettingsCard>
    </>
  );
}

function DictionaryTab() {
  return (
    <SettingsCard
      title="Dictionary"
      icon={BookOpen}
      description="Words and phrases that should be preserved during cleanup. Names, acronyms, jargon, project-specific terms — they all get fed to the LLM as context."
    >
      <DictionarySection />
    </SettingsCard>
  );
}

function SnippetsTab() {
  return (
    <SettingsCard
      title="Snippets"
      icon={Sparkles}
      description="Voice triggers that paste predefined text. Bypass post-processing — say it once, use it forever."
    >
      <SnippetsSection />
    </SettingsCard>
  );
}

function HistoryTab() {
  return (
    <SettingsCard
      title="Run Log"
      icon={History}
      description="Every dictation pass, searchable. Stored locally."
    >
      <HistorySection />
    </SettingsCard>
  );
}

function PromptsTab() {
  return (
    <SettingsCard
      title="Prompt instructions"
      icon={Sparkles}
      description="Append your own cleanup rules without editing the bundled system prompt."
    >
      <PromptSettingsSection />
    </SettingsCard>
  );
}

function AboutTab() {
  return (
    <div className="space-y-3">
      <div className="flex flex-col items-center rounded-lg border border-border/60 bg-card/40 px-6 py-8 text-center">
        <img
          src={logoMark}
          alt="wysprflow"
          className="size-16 rounded-2xl border border-border/60 shadow-sm"
        />
        <h2 className="mt-4 text-lg font-semibold tracking-tight">wysprflow</h2>
        <p className="text-xs text-muted-foreground">v0.1.0</p>
        <p className="mt-3 max-w-sm text-xs text-muted-foreground">
          Premium voice dictation for macOS. Open source. Built to surpass Wispr Flow on speed,
          footprint, and privacy.
        </p>
        <a
          href="https://github.com"
          target="_blank"
          rel="noreferrer"
          className="mt-4 inline-flex items-center gap-2 rounded-md border border-border bg-background px-3 py-1.5 text-xs font-medium hover:bg-accent"
        >
          <Github className="size-3.5" />
          View on GitHub
        </a>
      </div>
      <SettingsCard title="License" icon={Info}>
        <p className="text-xs text-muted-foreground">
          MIT licensed. © 2026 Usman Okayani. See <span className="font-mono">LICENSE</span> in the
          repository for details.
        </p>
      </SettingsCard>
    </div>
  );
}
