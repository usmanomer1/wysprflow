import { useEffect, useState, type ReactNode } from "react";
import { CheckCircle2, Loader2, Mic, Shield } from "lucide-react";
import { toast } from "sonner";

import { useApp } from "@/lib/store";
import {
  tauri,
  type AudioInputDevice,
  type DictationConfig,
  type PermissionState,
  type PermissionStatus,
} from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";

const INPUT_LANGUAGES = [
  { value: "auto", label: "Auto detect" },
  { value: "en", label: "English" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "nl", label: "Dutch" },
  { value: "hi", label: "Hindi" },
  { value: "ur", label: "Urdu" },
] as const;

const OUTPUT_LANGUAGES = [
  { value: "same", label: "Keep source language" },
  { value: "English", label: "English" },
  { value: "Spanish", label: "Spanish" },
  { value: "French", label: "French" },
  { value: "German", label: "German" },
  { value: "Italian", label: "Italian" },
  { value: "Portuguese", label: "Portuguese" },
  { value: "Hindi", label: "Hindi" },
  { value: "Urdu", label: "Urdu" },
] as const;

type SupportedLlmProvider = "openrouter" | "anthropic" | "off";

export function ProviderRoutingSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);

  if (!config) return null;

  const llmProvider = normalizeLlmProvider(config.llmProvider);

  const onProviderChange = async (next: SupportedLlmProvider) => {
    const nextModel = next === "openrouter" ? "anthropic/claude-haiku-4.5" : "claude-haiku-4-5";
    await setConfig({
      llmProvider: next,
      llmModel: next === "off" ? config.llmModel : nextModel,
    });
  };

  return (
    <div className="space-y-4">
      <Field
        label="Speech engine"
        hint="Deepgram Nova-3 is the live streaming engine in this build."
      >
        <div className="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-sm">
          Deepgram Nova-3
        </div>
      </Field>

      <div className="grid gap-4 md:grid-cols-2">
        <Field
          label="Cleanup provider"
          hint="Choose the model path that cleans up dictation before it is pasted."
        >
          <Select value={llmProvider} onValueChange={(value) => onProviderChange(value as SupportedLlmProvider)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="openrouter">OpenRouter</SelectItem>
              <SelectItem value="anthropic">Anthropic direct</SelectItem>
              <SelectItem value="off">Off</SelectItem>
            </SelectContent>
          </Select>
        </Field>

        <Field
          label="Cleanup intensity"
          hint="`None` skips stylistic cleanup but can still run translation or custom instructions."
        >
          <Select
            value={config.autoCleanup}
            onValueChange={(value) => setConfig({ autoCleanup: value as DictationConfig["autoCleanup"] })}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">None</SelectItem>
              <SelectItem value="light">Light</SelectItem>
              <SelectItem value="medium">Medium</SelectItem>
              <SelectItem value="high">High</SelectItem>
            </SelectContent>
          </Select>
        </Field>
      </div>

      <Field
        label="Cleanup model"
        hint={
          llmProvider === "openrouter"
            ? "Use a provider/model id such as `anthropic/claude-haiku-4.5`."
            : llmProvider === "anthropic"
              ? "Anthropic model id, defaulting to `claude-haiku-4-5`."
              : "Stored here so you can re-enable cleanup later without retyping it."
        }
      >
        <Input
          value={config.llmModel}
          onChange={(e) => setConfig({ llmModel: e.target.value })}
          className="font-mono text-sm"
          placeholder={llmProvider === "openrouter" ? "anthropic/claude-haiku-4.5" : "claude-haiku-4-5"}
        />
      </Field>
    </div>
  );
}

export function VoiceSettingsSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const loadDevices = async () => {
      try {
        const list = await tauri.listAudioInputDevices();
        if (!cancelled) setDevices(list);
      } catch (e) {
        if (!cancelled) toast.error(`Couldn't load microphones: ${String(e)}`);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    loadDevices();
    return () => {
      cancelled = true;
    };
  }, []);

  const selectedDevice =
    config && config.microphoneDevice !== "default" && devices.some((device) => device.id === config.microphoneDevice)
      ? config.microphoneDevice
      : "default";

  if (!config) return null;

  return (
    <div className="space-y-4">
      <div className="grid gap-4 md:grid-cols-2">
        <Field
          label="Microphone"
          hint="Pick a specific input device or stay on the system default."
        >
          <Select
            value={selectedDevice}
            onValueChange={(value) => setConfig({ microphoneDevice: value })}
            disabled={loading}
          >
            <SelectTrigger>
              <SelectValue placeholder={loading ? "Loading microphones…" : "Choose microphone"} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="default">System default</SelectItem>
              {devices.map((device) => (
                <SelectItem key={device.id} value={device.id}>
                  {device.name}
                  {device.isDefault ? " (default)" : ""}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field
          label="Recognition language"
          hint="Pin the input language when auto-detect is unstable."
        >
          <Select
            value={config.language}
            onValueChange={(value) => setConfig({ language: value })}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {INPUT_LANGUAGES.map((language) => (
                <SelectItem key={language.value} value={language.value}>
                  {language.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      </div>

      <div className="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
        Device changes apply on the next dictation start.
      </div>
    </div>
  );
}

export function PermissionsSection() {
  const [permissions, setPermissions] = useState<PermissionStatus | null>(null);
  const [busy, setBusy] = useState<"microphone" | "accessibility" | "inputMonitoring" | null>(null);

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      try {
        const next = await tauri.checkPermissions();
        if (!cancelled) setPermissions(next);
      } catch {
        /* backend may be starting */
      }
    };
    refresh();
    const timer = window.setInterval(refresh, 1500);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const runPermissionAction = async (
    key: "microphone" | "accessibility" | "inputMonitoring",
    action: () => Promise<void | boolean>,
  ) => {
    setBusy(key);
    try {
      await action();
      const next = await tauri.checkPermissions();
      setPermissions(next);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-3">
      <PermissionRow
        icon={Mic}
        label="Microphone"
        state={permissions?.microphone ?? "notDetermined"}
        hint="Needed to capture audio for dictation."
        actionLabel="Grant"
        busy={busy === "microphone"}
        onAction={() =>
          runPermissionAction("microphone", async () => {
            if ((permissions?.microphone ?? "notDetermined") === "notDetermined") {
              await tauri.requestMicrophone();
            } else {
              await tauri.openMicrophoneSettings();
            }
          })
        }
      />
      <PermissionRow
        icon={Shield}
        label="Accessibility"
        state={permissions?.accessibility ?? "notDetermined"}
        hint="Needed to paste into the focused app."
        actionLabel="Open Settings"
        busy={busy === "accessibility"}
        onAction={() => runPermissionAction("accessibility", tauri.openAccessibilitySettings)}
      />
      <PermissionRow
        icon={Shield}
        label="Input Monitoring"
        state={permissions?.inputMonitoring ?? "notDetermined"}
        hint="Needed for the Fn-key hotkey path."
        actionLabel="Open Settings"
        busy={busy === "inputMonitoring"}
        optional
        onAction={() => runPermissionAction("inputMonitoring", tauri.openInputMonitoringSettings)}
      />
    </div>
  );
}

export function BehaviorSettingsSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);

  if (!config) return null;

  return (
    <div className="space-y-3">
      <ToggleRow
        label="Restore clipboard after paste"
        hint="Keeps your previous clipboard contents intact after dictation."
        checked={config.preserveClipboard}
        onCheckedChange={(checked) => setConfig({ preserveClipboard: checked })}
      />
      <ToggleRow
        label="Expand snippets"
        hint="Allows spoken snippet triggers to replace the cleaned transcript."
        checked={config.snippetsEnabled}
        onCheckedChange={(checked) => setConfig({ snippetsEnabled: checked })}
      />
      <ToggleRow
        label="Play start / stop sounds"
        hint="Uses the system alert sounds when dictation begins and ends."
        checked={config.playSounds}
        onCheckedChange={(checked) => setConfig({ playSounds: checked })}
      />
    </div>
  );
}

export function TranslationSettingsSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);

  if (!config) return null;

  const cleanupDisabled = normalizeLlmProvider(config.llmProvider) === "off";

  return (
    <div className="space-y-3">
      <Field
        label="Paste language"
        hint="Translate the final pasted output after transcription when cleanup is enabled."
      >
        <Select
          value={config.translateTo}
          onValueChange={(value) => setConfig({ translateTo: value })}
          disabled={cleanupDisabled}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {OUTPUT_LANGUAGES.map((language) => (
              <SelectItem key={language.value} value={language.value}>
                {language.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      {cleanupDisabled ? (
        <div className="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
          Translation needs a cleanup provider. Turn cleanup back on to enable translated paste.
        </div>
      ) : null}
    </div>
  );
}

export function StartupSettingsSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);
  const [busy, setBusy] = useState(false);

  if (!config) return null;

  const onCheckedChange = async (checked: boolean) => {
    setBusy(true);
    try {
      await setConfig({ launchAtLogin: checked });
    } catch (e) {
      toast.error(`Couldn't update startup preference: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <ToggleRow
      label="Launch at login"
      hint="Creates or removes a macOS LaunchAgent. Changes apply on the next login."
      checked={config.launchAtLogin}
      disabled={busy}
      indicator={busy ? <Loader2 className="size-3.5 animate-spin" /> : undefined}
      onCheckedChange={onCheckedChange}
    />
  );
}

export function PromptSettingsSection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setDraft(config?.customCleanupPrompt ?? "");
  }, [config?.customCleanupPrompt]);

  if (!config) return null;

  const save = async () => {
    setBusy(true);
    try {
      await setConfig({ customCleanupPrompt: draft.trim() });
      toast.success("Prompt instructions saved");
    } catch (e) {
      toast.error(`Couldn't save prompt instructions: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const isDirty = draft !== (config.customCleanupPrompt ?? "");

  return (
    <div className="space-y-3">
      <Field
        label="Additional cleanup instructions"
        hint="Extra rules appended to the dictation cleanup system prompt."
      >
        <Textarea
          rows={6}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Example: keep my Slack messages casual, use sentence case, never add em dashes."
        />
      </Field>
      <div className="flex justify-end gap-2">
        <Button
          variant="ghost"
          disabled={busy || (!draft && !config.customCleanupPrompt)}
          onClick={() => setDraft("")}
        >
          Clear
        </Button>
        <Button onClick={save} disabled={!isDirty || busy}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : "Save instructions"}
        </Button>
      </div>
    </div>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="space-y-0.5">
        <Label>{label}</Label>
        {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
      </div>
      {children}
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  disabled,
  indicator,
  onCheckedChange,
}: {
  label: string;
  hint: string;
  checked: boolean;
  disabled?: boolean;
  indicator?: ReactNode;
  onCheckedChange: (checked: boolean) => void | Promise<void>;
}) {
  return (
    <div className="flex items-center gap-3 rounded-md border border-border/60 bg-background/70 px-3 py-3">
      <div className="flex-1 space-y-0.5">
        <div className="text-sm font-medium">{label}</div>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </div>
      {indicator}
      <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
    </div>
  );
}

function PermissionRow({
  icon: Icon,
  label,
  hint,
  state,
  actionLabel,
  optional,
  busy,
  onAction,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  hint: string;
  state: PermissionState;
  actionLabel: string;
  optional?: boolean;
  busy?: boolean;
  onAction: () => void;
}) {
  const granted = state === "granted";

  return (
    <div className="flex items-center gap-3 rounded-md border border-border/60 bg-background/70 px-3 py-3">
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <div className="flex-1 space-y-0.5">
        <div className="flex items-center gap-2 text-sm font-medium">
          {label}
          {optional ? (
            <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground">
              Optional
            </span>
          ) : null}
        </div>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </div>
      {granted ? (
        <div className="inline-flex items-center gap-1 text-xs text-emerald-500">
          <CheckCircle2 className="size-3.5" /> Granted
        </div>
      ) : (
        <Button size="sm" variant="outline" disabled={busy} onClick={onAction}>
          {busy ? <Loader2 className="size-3.5 animate-spin" /> : actionLabel}
        </Button>
      )}
    </div>
  );
}

function normalizeLlmProvider(provider: DictationConfig["llmProvider"]): SupportedLlmProvider {
  return provider === "anthropic" || provider === "openrouter" ? provider : "off";
}
