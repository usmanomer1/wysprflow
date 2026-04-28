import { useEffect, useState } from "react";
import { toast } from "sonner";

import { useApp } from "@/lib/store";
import { tauri } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

/**
 * Captures a keyboard accelerator string in Tauri format (e.g. "CmdOrCtrl+Shift+Space").
 * For the Fn key on Mac we ship a separate native CGEventTap path — see hotkey/fn_key.rs.
 */
function recordAccelerator(e: React.KeyboardEvent<HTMLInputElement>): string | null {
  const parts: string[] = [];
  if (e.metaKey) parts.push("Cmd");
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");

  const key = e.key;
  if (["Meta", "Control", "Alt", "Shift"].includes(key)) return null;

  let mapped = key.length === 1 ? key.toUpperCase() : key;
  if (mapped === " ") mapped = "Space";
  if (mapped === "Escape") return null;
  parts.push(mapped);

  return parts.join("+");
}

export function HotkeySection() {
  const config = useApp((s) => s.config);
  const setConfig = useApp((s) => s.setConfig);

  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);

  useEffect(() => {
    setDraft(config?.hotkey ?? null);
  }, [config?.hotkey]);

  if (!config) return null;

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const accel = recordAccelerator(e);
    if (accel) {
      setDraft(accel);
      setRecording(false);
    }
  };

  const onSave = async () => {
    if (!draft || draft === config.hotkey) return;
    try {
      await tauri.setHotkey(draft);
      await setConfig({ hotkey: draft });
      toast.success("Hotkey updated");
    } catch (e) {
      toast.error(`Couldn't register hotkey: ${String(e)}`);
    }
  };

  return (
    <div className="space-y-4">
      <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
        <div className="space-y-2">
          <Label>Trigger mode</Label>
          <Select
            value={config.hotkeyMode}
            onValueChange={(value) => setConfig({ hotkeyMode: value as typeof config.hotkeyMode })}
          >
            <SelectTrigger className="w-full md:w-72">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="hold">Hold to dictate (push-to-talk)</SelectItem>
              <SelectItem value="toggle">Tap once to toggle</SelectItem>
              <SelectItem value="tap-toggle">Double-tap to toggle</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
          Fn-key dictation is the default macOS path. Standard keyboard shortcuts remain
          available if Input Monitoring or Globe/Fn behavior gets in the way.
        </div>
      </div>

      <div className="space-y-2">
        <Label>Keyboard shortcut</Label>
        <div className="flex flex-col gap-2 sm:flex-row">
          <input
            readOnly
            value={recording ? "Press a key combo…" : draft ?? ""}
            placeholder="No shortcut set"
            className="flex h-9 w-full items-center rounded-md border border-input bg-transparent px-3 py-1 font-mono text-sm focus:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:max-w-72"
            onFocus={() => setRecording(true)}
            onBlur={() => setRecording(false)}
            onKeyDown={handleKeyDown}
          />
          <Button onClick={onSave} disabled={!draft || draft === config.hotkey}>
            Save
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Pick a shortcut that won&apos;t collide with system shortcuts.
        </p>
      </div>
    </div>
  );
}
