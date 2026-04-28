import { useEffect, useMemo, useState } from "react";
import {
  Mic,
  Sparkles,
  KeyRound,
  LockKeyhole,
  CheckCircle2,
  ArrowRight,
  ArrowLeft,
  Loader2,
  ExternalLink,
  Eye,
  EyeOff,
} from "lucide-react";
import { toast } from "sonner";

import { tauri, type PermissionStatus, type ProviderKey } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

type StepId = "welcome" | "keys" | "permissions" | "done";
const STEPS: StepId[] = ["welcome", "keys", "permissions", "done"];

type LlmProvider = "openrouter" | "anthropic";

export default function Setup({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState<StepId>("welcome");
  const [perms, setPerms] = useState<PermissionStatus | null>(null);
  const [keysOk, setKeysOk] = useState({
    deepgram: false,
    openrouter: false,
    anthropic: false,
  });
  const [llmProvider, setLlmProvider] = useState<LlmProvider>("openrouter");

  // Sync the picker with whatever's already in config (re-running setup keeps choice).
  useEffect(() => {
    tauri
      .getConfig()
      .then((cfg) => {
        if (cfg.llmProvider === "anthropic" || cfg.llmProvider === "openrouter") {
          setLlmProvider(cfg.llmProvider);
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    let cancelled = false;

    const refreshPermissions = async () => {
      try {
        const p = await tauri.checkPermissions();
        if (!cancelled) setPerms(p);
      } catch {
        /* backend may not be ready */
      }
    };

    const refreshKeys = async () => {
      try {
        const statuses = await tauri.listApiKeyStatuses();
        if (!cancelled) {
          setKeysOk({
            deepgram: !!statuses.find((s) => s.provider === "deepgram")?.hasKey,
            openrouter: !!statuses.find((s) => s.provider === "openrouter")?.hasKey,
            anthropic: !!statuses.find((s) => s.provider === "anthropic")?.hasKey,
          });
        }
      } catch {
        /* backend may not be ready */
      }
    };

    refreshPermissions();
    refreshKeys();

    const t = setInterval(refreshPermissions, 1500);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  const idx = STEPS.indexOf(step);
  const goNext = () => {
    if (idx < STEPS.length - 1) setStep(STEPS[idx + 1]);
  };
  const goBack = () => {
    if (idx > 0) setStep(STEPS[idx - 1]);
  };

  const finish = async () => {
    try {
      await tauri.updateConfig({ setupCompleted: true });
      onComplete();
    } catch (e) {
      toast.error(`Couldn't save: ${e}`);
    }
  };

  const llmKeyOk = llmProvider === "openrouter" ? keysOk.openrouter : keysOk.anthropic;

  const canAdvance = useMemo(() => {
    if (step === "keys") return keysOk.deepgram && llmKeyOk;
    if (step === "permissions") {
      return perms?.microphone === "granted" && perms?.accessibility === "granted";
    }
    return true;
  }, [step, keysOk, llmKeyOk, perms]);

  return (
    <div className="flex h-full flex-col bg-background">
      <header
        data-tauri-drag-region
        className="flex h-10 shrink-0 items-center justify-end border-b border-border/40 px-3"
      >
        <button
          onClick={() => finish()}
          className="text-[11px] text-muted-foreground hover:text-foreground"
        >
          Skip setup
        </button>
      </header>

      <main className="flex flex-1 items-center justify-center px-10">
        <div className="w-full max-w-md">
          {step === "welcome" && <WelcomeStep />}
          {step === "keys" && (
            <KeysStep
              keysOk={keysOk}
              llmProvider={llmProvider}
              onKeysChanged={async () => {
                const statuses = await tauri.listApiKeyStatuses();
                setKeysOk({
                  deepgram: !!statuses.find((s) => s.provider === "deepgram")?.hasKey,
                  openrouter: !!statuses.find((s) => s.provider === "openrouter")?.hasKey,
                  anthropic: !!statuses.find((s) => s.provider === "anthropic")?.hasKey,
                });
              }}
              onPickLlm={async (next) => {
                setLlmProvider(next);
                try {
                  await tauri.updateConfig({ llmProvider: next });
                } catch {
                  /* ignore */
                }
              }}
            />
          )}
          {step === "permissions" && <PermissionsStep perms={perms} />}
          {step === "done" && <DoneStep />}
        </div>
      </main>

      <footer className="flex shrink-0 items-center justify-between border-t border-border/40 px-6 py-4">
        <Button
          variant="ghost"
          onClick={goBack}
          disabled={idx === 0}
          className={cn(idx === 0 && "invisible")}
        >
          <ArrowLeft className="size-3.5" /> Back
        </Button>

        <DotIndicator current={idx} total={STEPS.length} />

        {step === "done" ? (
          <Button onClick={finish}>
            Get started <ArrowRight className="size-3.5" />
          </Button>
        ) : (
          <Button onClick={goNext} disabled={!canAdvance}>
            Continue <ArrowRight className="size-3.5" />
          </Button>
        )}
      </footer>
    </div>
  );
}

function DotIndicator({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex items-center gap-1.5">
      {Array.from({ length: total }).map((_, i) => (
        <span
          key={i}
          className={cn(
            "size-1.5 rounded-full transition-colors",
            i === current ? "bg-foreground" : "bg-foreground/20",
          )}
        />
      ))}
    </div>
  );
}

function WelcomeStep() {
  return (
    <div className="space-y-5 text-center">
      <div className="mx-auto flex size-20 items-center justify-center rounded-3xl bg-foreground/10">
        <Mic className="size-9" />
      </div>
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight">Welcome to wysprflow</h1>
        <p className="text-sm text-muted-foreground">
          Hold a hotkey, talk into any app, and your words appear cleaned-up at your cursor.
          Open-source. Bring your own keys. Your audio never touches our servers — there are
          no servers.
        </p>
      </div>
      <p className="text-xs text-muted-foreground">
        This setup will take less than a minute.
      </p>
    </div>
  );
}

function KeysStep({
  keysOk,
  llmProvider,
  onKeysChanged,
  onPickLlm,
}: {
  keysOk: { deepgram: boolean; openrouter: boolean; anthropic: boolean };
  llmProvider: LlmProvider;
  onKeysChanged: () => Promise<void>;
  onPickLlm: (p: LlmProvider) => Promise<void>;
}) {
  return (
    <div className="space-y-5">
      <div className="space-y-2 text-center">
        <div className="mx-auto flex size-16 items-center justify-center rounded-2xl bg-foreground/10">
          <KeyRound className="size-7" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">API keys</h1>
        <p className="text-sm text-muted-foreground">
          wysprflow talks directly to your providers. In development, keys are stored locally. In
          packaged macOS builds, they live in Keychain.
        </p>
      </div>

      <div className="space-y-3">
        <KeyRow
          provider="deepgram"
          label="Deepgram"
          hint="Streaming speech-to-text (Nova-3)"
          url="https://console.deepgram.com/project/keys"
          placeholder="Token from Deepgram console"
          ok={keysOk.deepgram}
          onStored={onKeysChanged}
        />

        {/* LLM provider toggle */}
        <div className="space-y-2 pt-1">
          <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            LLM cleanup provider
          </p>
          <div className="grid grid-cols-2 gap-2">
            <ProviderTile
              active={llmProvider === "openrouter"}
              onClick={() => onPickLlm("openrouter")}
              title="OpenRouter"
              hint="Single key, any model"
            />
            <ProviderTile
              active={llmProvider === "anthropic"}
              onClick={() => onPickLlm("anthropic")}
              title="Anthropic"
              hint="Claude Haiku 4.5 direct"
            />
          </div>
        </div>

        {llmProvider === "openrouter" ? (
          <KeyRow
            provider="openrouter"
            label="OpenRouter"
            hint="Routes anthropic/claude-haiku-4.5 by default"
            url="https://openrouter.ai/settings/keys"
            placeholder="sk-or-v1-..."
            ok={keysOk.openrouter}
            onStored={onKeysChanged}
          />
        ) : (
          <KeyRow
            provider="anthropic"
            label="Anthropic"
            hint="Cleanup pass with Claude Haiku 4.5"
            url="https://console.anthropic.com/settings/keys"
            placeholder="sk-ant-..."
            ok={keysOk.anthropic}
            onStored={onKeysChanged}
          />
        )}
      </div>

      <p className="text-center text-xs text-muted-foreground">
        Both keys are required. You can add Groq, OpenAI, or ElevenLabs later from Settings.
      </p>
    </div>
  );
}

function ProviderTile({
  active,
  onClick,
  title,
  hint,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  hint: string;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-md border px-3 py-2 text-left transition-colors",
        active
          ? "border-foreground bg-foreground/5"
          : "border-border/60 hover:bg-card/40",
      )}
    >
      <div className="text-sm font-medium">{title}</div>
      <div className="text-[11px] text-muted-foreground">{hint}</div>
    </button>
  );
}

function KeyRow({
  provider,
  label,
  hint,
  url,
  placeholder,
  ok,
  onStored,
}: {
  provider: ProviderKey;
  label: string;
  hint: string;
  url: string;
  placeholder: string;
  ok: boolean;
  onStored: () => Promise<void>;
}) {
  const [draft, setDraft] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);

  const onSave = async () => {
    if (!draft.trim()) return;
    setBusy(true);
    try {
      await tauri.setApiKey(provider, draft.trim());
      setDraft("");
      try {
        await onStored();
      } catch {
        /* key was still saved; don't fail the flow on a status refresh miss */
      }
      const result = await tauri.validateApiKey(provider);
      if (result.ok) toast.success(`${label} key validated`);
      else toast.error(`${label} validation failed: ${result.detail ?? "unknown"}`);
    } catch (e) {
      toast.error(`Save failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-md border border-border/60 bg-card/40 p-3">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-0.5">
          <div className="flex items-center gap-1.5 text-sm font-medium">
            {label}
            {ok ? <CheckCircle2 className="size-3.5 text-emerald-500" /> : null}
          </div>
          <p className="text-xs text-muted-foreground">{hint}</p>
        </div>
        <a
          href={url}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground"
        >
          Get key <ExternalLink className="size-3" />
        </a>
      </div>

      {!ok && (
        <div className="mt-3 flex gap-2">
          <div className="relative flex-1">
            <Input
              type={show ? "text" : "password"}
              placeholder={placeholder}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onSave()}
              className="pr-9 font-mono text-xs"
            />
            <button
              type="button"
              onClick={() => setShow((v) => !v)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              tabIndex={-1}
            >
              {show ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
            </button>
          </div>
          <Button onClick={onSave} disabled={!draft.trim() || busy} size="sm">
            {busy ? <Loader2 className="size-3.5 animate-spin" /> : "Save"}
          </Button>
        </div>
      )}
    </div>
  );
}

function PermissionsStep({ perms }: { perms: PermissionStatus | null }) {
  return (
    <div className="space-y-5">
      <div className="space-y-2 text-center">
        <div className="mx-auto flex size-16 items-center justify-center rounded-2xl bg-foreground/10">
          <LockKeyhole className="size-7" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">Grant access</h1>
        <p className="text-sm text-muted-foreground">
          macOS asks for these the first time wysprflow needs them. We poll for status — toggle
          the permission and it'll flip green here.
        </p>
      </div>

      <div className="space-y-2">
        <PermissionRow
          icon={Mic}
          label="Microphone"
          hint="Required to capture audio for transcription."
          state={perms?.microphone ?? "notDetermined"}
          onGrant={async () => {
            await tauri.requestMicrophone();
          }}
          openSettings={() => tauri.openMicrophoneSettings()}
        />
        <PermissionRow
          icon={LockKeyhole}
          label="Accessibility"
          hint="Required to inject text into other apps via Cmd+V."
          state={perms?.accessibility ?? "notDetermined"}
          openSettings={() => tauri.openAccessibilitySettings()}
        />
        <PermissionRow
          icon={Sparkles}
          label="Input Monitoring"
          hint="Required for the Fn-key hotkey. Optional — Cmd+Shift+Space works without it."
          state={perms?.inputMonitoring ?? "notDetermined"}
          optional
          openSettings={() => tauri.openInputMonitoringSettings()}
        />
      </div>
    </div>
  );
}

function PermissionRow({
  icon: Icon,
  label,
  hint,
  state,
  optional = false,
  onGrant,
  openSettings,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  hint: string;
  state: "granted" | "denied" | "notDetermined";
  optional?: boolean;
  onGrant?: () => Promise<void>;
  openSettings: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);

  const handleClick = async () => {
    setBusy(true);
    try {
      if (onGrant && state === "notDetermined") {
        await onGrant();
      } else {
        await openSettings();
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex items-center gap-3 rounded-md border border-border/60 bg-card/40 px-3 py-2.5">
      <Icon className="size-4 shrink-0" />
      <div className="flex-1 space-y-0.5 overflow-hidden">
        <div className="flex items-center gap-1.5 text-sm font-medium">
          {label}
          {optional ? (
            <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] uppercase text-muted-foreground">
              Optional
            </span>
          ) : null}
        </div>
        <p className="text-[11px] text-muted-foreground">{hint}</p>
      </div>
      {state === "granted" ? (
        <span className="inline-flex items-center gap-1 text-xs text-emerald-500">
          <CheckCircle2 className="size-3.5" /> Granted
        </span>
      ) : (
        <Button size="sm" variant="outline" onClick={handleClick} disabled={busy}>
          {busy ? <Loader2 className="size-3.5 animate-spin" /> : "Grant"}
        </Button>
      )}
    </div>
  );
}

function DoneStep() {
  return (
    <div className="space-y-5 text-center">
      <div className="mx-auto flex size-20 items-center justify-center rounded-3xl bg-emerald-500/15">
        <CheckCircle2 className="size-10 text-emerald-500" />
      </div>
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight">You're all set</h1>
        <p className="text-sm text-muted-foreground">
          Hold <kbd className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">Fn</kbd> (or
          your configured shortcut) anywhere on your Mac and start talking. Your cleaned
          transcript pastes at the cursor when you release.
        </p>
      </div>
      <div className="rounded-md border border-border/60 bg-card/40 p-3 text-left">
        <p className="text-[11px] uppercase tracking-wider text-muted-foreground">
          Tip if Fn opens Emoji
        </p>
        <p className="mt-1 text-xs text-muted-foreground">
          System Settings → Keyboard → "Press Fn key to" → set to <em>Do Nothing</em>.
        </p>
      </div>
    </div>
  );
}
