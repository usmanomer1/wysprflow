import { useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  Eye,
  EyeOff,
  Loader2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";

import type { ProviderKey } from "@/lib/tauri";
import { useApp } from "@/lib/store";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { cn } from "@/lib/utils";

interface ProviderMeta {
  provider: ProviderKey;
  name: string;
  blurb: string;
  url: string;
  placeholder: string;
  testable: boolean;
}

const PROVIDERS: ProviderMeta[] = [
  {
    provider: "deepgram",
    name: "Deepgram",
    blurb: "Nova-3 streaming speech-to-text",
    url: "https://console.deepgram.com/project/keys",
    placeholder: "Token from Deepgram console",
    testable: true,
  },
  {
    provider: "openrouter",
    name: "OpenRouter",
    blurb: "Cleanup provider with broad model routing",
    url: "https://openrouter.ai/settings/keys",
    placeholder: "sk-or-v1-...",
    testable: true,
  },
  {
    provider: "anthropic",
    name: "Anthropic",
    blurb: "Direct Claude cleanup provider",
    url: "https://console.anthropic.com/settings/keys",
    placeholder: "sk-ant-...",
    testable: true,
  },
  {
    provider: "groq",
    name: "Groq",
    blurb: "Stored for future STT support",
    url: "https://console.groq.com/keys",
    placeholder: "gsk_...",
    testable: false,
  },
  {
    provider: "openai",
    name: "OpenAI",
    blurb: "Stored for future provider support",
    url: "https://platform.openai.com/api-keys",
    placeholder: "sk-...",
    testable: false,
  },
  {
    provider: "elevenlabs",
    name: "ElevenLabs",
    blurb: "Stored for future provider support",
    url: "https://elevenlabs.io/app/settings/api-keys",
    placeholder: "elv_...",
    testable: false,
  },
];

const PROVIDER_META = Object.fromEntries(
  PROVIDERS.map((provider) => [provider.provider, provider]),
) as Record<ProviderKey, ProviderMeta>;

export function ApiKeysSection() {
  const config = useApp((s) => s.config);
  const keys = useApp((s) => s.keys);
  const [selectedProvider, setSelectedProvider] = useState<ProviderKey>("deepgram");

  const activeProviders = useMemo(() => {
    const providers: ProviderKey[] = ["deepgram"];
    if (config?.llmProvider === "openrouter" || config?.llmProvider === "anthropic") {
      providers.push(config.llmProvider);
    }
    return Array.from(new Set(providers));
  }, [config?.llmProvider]);

  const storedProviders = useMemo(
    () => PROVIDERS.filter((provider) => keys[provider.provider].hasKey),
    [keys],
  );

  return (
    <div className="space-y-4">
      <div className="rounded-md border border-border/60 bg-muted/20 p-3">
        <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div className="space-y-1">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Active providers
            </p>
            <p className="text-xs text-muted-foreground">
              Keep the keys for the providers you actually use front and center. Everything else
              lives behind the selector below.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {activeProviders.map((provider) => (
              <button
                key={provider}
                type="button"
                onClick={() => setSelectedProvider(provider)}
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs transition-colors",
                  selectedProvider === provider
                    ? "border-foreground/30 bg-foreground/5 text-foreground"
                    : "border-border/60 text-muted-foreground hover:text-foreground",
                )}
              >
                <ProviderStatusDot provider={provider} />
                {PROVIDER_META[provider].name}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_16rem] md:items-end">
        <div className="space-y-2">
          <Label>Manage provider key</Label>
          <Select
            value={selectedProvider}
            onValueChange={(value) => setSelectedProvider(value as ProviderKey)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PROVIDERS.map((provider) => (
                <SelectItem key={provider.provider} value={provider.provider}>
                  {provider.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="rounded-md border border-border/60 bg-background/70 px-3 py-2">
          <div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Stored keys
          </div>
          <div className="mt-1 flex min-h-6 flex-wrap gap-1.5">
            {storedProviders.length > 0 ? (
              storedProviders.map((provider) => (
                <button
                  key={provider.provider}
                  type="button"
                  onClick={() => setSelectedProvider(provider.provider)}
                  className="rounded bg-muted px-2 py-1 text-[11px] text-foreground/80 hover:bg-muted/80"
                >
                  {provider.name}
                </button>
              ))
            ) : (
              <span className="text-xs text-muted-foreground">No keys saved yet</span>
            )}
          </div>
        </div>
      </div>

      <ProviderEditor provider={selectedProvider} />
    </div>
  );
}

function ProviderEditor({ provider }: { provider: ProviderKey }) {
  const meta = PROVIDER_META[provider];
  const status = useApp((s) => s.keys[provider]);
  const setKey = useApp((s) => s.setKey);
  const removeKey = useApp((s) => s.removeKey);
  const validateKey = useApp((s) => s.validateKey);

  const [draft, setDraft] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState<"save" | "validate" | "delete" | null>(null);

  useEffect(() => {
    setDraft("");
    setShow(false);
    setBusy(null);
  }, [provider]);

  const onSave = async () => {
    if (!draft.trim()) return;
    setBusy("save");
    try {
      await setKey(provider, draft.trim());
      setDraft("");
      toast.success(`${meta.name} key saved`);
      if (meta.testable) {
        const result = await validateKey(provider);
        if (result.ok) toast.success(`${meta.name} key validated`);
        else toast.error(`${meta.name} validation failed: ${result.detail ?? "unknown error"}`);
      }
    } catch (e) {
      toast.error(`Failed to save: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  const onValidate = async () => {
    setBusy("validate");
    try {
      const result = await validateKey(provider);
      if (result.ok) toast.success(`${meta.name} key works`);
      else toast.error(`${meta.name} validation failed: ${result.detail ?? "unknown"}`);
    } finally {
      setBusy(null);
    }
  };

  const onRemove = async () => {
    setBusy("delete");
    try {
      await removeKey(provider);
      toast.success(`${meta.name} key removed`);
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="rounded-lg border border-border/60 bg-background/80 p-4">
      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <h4 className="text-sm font-semibold">{meta.name}</h4>
            {status.hasKey && status.validatedAt ? (
              <CheckCircle2 className="size-4 text-emerald-500" />
            ) : status.hasKey ? (
              <AlertCircle className="size-4 text-amber-500" />
            ) : null}
          </div>
          <p className="text-xs text-muted-foreground">{meta.blurb}</p>
        </div>
        <a
          href={meta.url}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          Get key <ExternalLink className="size-3" />
        </a>
      </div>

      {status.hasKey ? (
        <div className="mt-4 flex flex-col gap-3 rounded-md border border-dashed border-border/60 px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-0.5">
            <div className="text-xs text-muted-foreground">
              {meta.testable
                ? status.validatedAt
                  ? `Validated ${new Date(status.validatedAt).toLocaleString()}`
                  : "Saved locally. Run a connection test when needed."
                : "Saved locally. This provider is not validated in this build yet."}
            </div>
            <div className="font-mono text-sm">••••••••••••••••</div>
          </div>
          <div className="flex gap-2">
            {meta.testable ? (
              <Button size="sm" variant="outline" onClick={onValidate} disabled={busy !== null}>
                {busy === "validate" ? <Loader2 className="size-3 animate-spin" /> : "Test"}
              </Button>
            ) : null}
            <Button size="sm" variant="ghost" onClick={onRemove} disabled={busy !== null}>
              {busy === "delete" ? (
                <Loader2 className="size-3 animate-spin" />
              ) : (
                <Trash2 className="size-3" />
              )}
            </Button>
          </div>
        </div>
      ) : (
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <div className="relative flex-1">
            <Input
              type={show ? "text" : "password"}
              placeholder={meta.placeholder}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onSave()}
              className="pr-9 font-mono text-sm"
            />
            <button
              type="button"
              onClick={() => setShow((value) => !value)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              tabIndex={-1}
            >
              {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </button>
          </div>
          <Button onClick={onSave} disabled={!draft.trim() || busy !== null}>
            {busy === "save" ? <Loader2 className="size-4 animate-spin" /> : "Save"}
          </Button>
        </div>
      )}
    </div>
  );
}

function ProviderStatusDot({ provider }: { provider: ProviderKey }) {
  const status = useApp((s) => s.keys[provider]);
  if (!status.hasKey) {
    return <span className="size-2 rounded-full bg-border" />;
  }
  if (status.validatedAt) {
    return <span className="size-2 rounded-full bg-emerald-500" />;
  }
  return <span className="size-2 rounded-full bg-amber-500" />;
}
