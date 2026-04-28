import { useEffect, useMemo, useState } from "react";
import { Search, Trash2, AlertCircle, Copy, ChevronRight, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { tauri, type HistoryEntry } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export function HistorySection() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [query, setQuery] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const refresh = async (q?: string) => {
    try {
      const search = q ?? query;
      const list = search.trim()
        ? await tauri.searchHistory(search.trim())
        : await tauri.listHistory();
      setEntries(list);
    } catch (e) {
      toast.error(`Couldn't load history: ${e}`);
    } finally {
      setLoaded(true);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const onDelete = async (id: string) => {
    try {
      await tauri.deleteHistoryEntry(id);
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
    }
  };

  const onClearAll = async () => {
    if (!confirm("Clear all history?")) return;
    try {
      await tauri.clearHistory();
      setEntries([]);
    } catch (e) {
      toast.error(`Clear failed: ${e}`);
    }
  };

  const onCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied");
    } catch (e) {
      toast.error(`Copy failed: ${e}`);
    }
  };

  const grouped = useMemo(() => groupByDay(entries), [entries]);

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search transcripts…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && refresh()}
            className="pl-8"
          />
        </div>
        <Button variant="outline" onClick={() => refresh()}>
          Search
        </Button>
        {entries.length > 0 && (
          <Button variant="ghost" onClick={onClearAll}>
            <Trash2 className="size-3.5" /> Clear all
          </Button>
        )}
      </div>

      {!loaded ? (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="size-3 animate-spin" /> Loading…
        </div>
      ) : entries.length === 0 ? (
        <div className="rounded border border-dashed border-border/60 bg-muted/20 p-6 text-center">
          <p className="text-xs text-muted-foreground">
            No dictation history yet. Hold your hotkey and start talking — every session lands
            here, searchable.
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {grouped.map(({ label, items }) => (
            <div key={label} className="space-y-1.5">
              <h4 className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                {label}
              </h4>
              {items.map((entry) => (
                <Entry
                  key={entry.id}
                  entry={entry}
                  expanded={expandedId === entry.id}
                  onToggle={() => setExpandedId(expandedId === entry.id ? null : entry.id)}
                  onCopy={onCopy}
                  onDelete={onDelete}
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Entry({
  entry,
  expanded,
  onToggle,
  onCopy,
  onDelete,
}: {
  entry: HistoryEntry;
  expanded: boolean;
  onToggle: () => void;
  onCopy: (text: string) => void;
  onDelete: (id: string) => void;
}) {
  const isError = !!entry.error;
  const time = new Date(entry.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  const wpm =
    entry.durationMs && entry.durationMs > 0
      ? Math.round((entry.wordCount / entry.durationMs) * 60_000)
      : null;

  return (
    <div
      className={cn(
        "rounded-md border bg-card/30 transition-colors",
        isError ? "border-destructive/40" : "border-border/40",
        expanded ? "bg-card/60" : "hover:bg-card/60",
      )}
    >
      <button
        onClick={onToggle}
        className="flex w-full items-start gap-2 px-3 py-2 text-left"
      >
        <ChevronRight
          className={cn(
            "mt-1 size-3 shrink-0 text-muted-foreground transition-transform",
            expanded && "rotate-90",
          )}
        />
        {isError ? <AlertCircle className="mt-0.5 size-3.5 shrink-0 text-destructive" /> : null}
        <div className="flex-1 space-y-0.5 overflow-hidden">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span className="font-mono">{time}</span>
            {entry.wordCount > 0 ? <span>{entry.wordCount} words</span> : null}
            {wpm !== null ? <span>{wpm} wpm</span> : null}
          </div>
          <div className="line-clamp-1 text-sm">
            {entry.error ?? entry.cleanedTranscript ?? "(empty)"}
          </div>
        </div>
        <div className="flex shrink-0 gap-1" onClick={(e) => e.stopPropagation()}>
          {entry.cleanedTranscript ? (
            <Button size="sm" variant="ghost" onClick={() => onCopy(entry.cleanedTranscript)}>
              <Copy className="size-3" />
            </Button>
          ) : null}
          <Button size="sm" variant="ghost" onClick={() => onDelete(entry.id)}>
            <Trash2 className="size-3" />
          </Button>
        </div>
      </button>

      {expanded ? (
        <div className="space-y-2 border-t border-border/40 px-3 py-2 text-xs">
          {entry.cleanedTranscript ? (
            <Stage label="Cleaned" body={entry.cleanedTranscript} />
          ) : null}
          {entry.rawTranscript && entry.rawTranscript !== entry.cleanedTranscript ? (
            <Stage label="Raw" body={entry.rawTranscript} muted />
          ) : null}
          {entry.error ? <Stage label="Error" body={entry.error} error /> : null}
        </div>
      ) : null}
    </div>
  );
}

function Stage({
  label,
  body,
  muted = false,
  error = false,
}: {
  label: string;
  body: string;
  muted?: boolean;
  error?: boolean;
}) {
  return (
    <div>
      <div className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
        {label}
      </div>
      <div
        className={cn(
          "mt-0.5 whitespace-pre-wrap break-words rounded bg-muted/40 p-2 font-mono text-xs",
          muted && "text-muted-foreground",
          error && "text-destructive",
        )}
      >
        {body}
      </div>
    </div>
  );
}

function groupByDay(entries: HistoryEntry[]) {
  const groups = new Map<string, HistoryEntry[]>();
  for (const entry of entries) {
    const date = new Date(entry.timestamp);
    const label = relativeDayLabel(date);
    const list = groups.get(label) ?? [];
    list.push(entry);
    groups.set(label, list);
  }
  return Array.from(groups.entries()).map(([label, items]) => ({ label, items }));
}

function relativeDayLabel(d: Date): string {
  const now = new Date();
  const startOf = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const todayMs = startOf(now).getTime();
  const dayMs = startOf(d).getTime();
  const dayDiff = Math.floor((todayMs - dayMs) / (24 * 60 * 60 * 1000));
  if (dayDiff === 0) return "Today";
  if (dayDiff === 1) return "Yesterday";
  if (dayDiff < 7) return "This week";
  if (dayDiff < 14) return "Last week";
  return "Older";
}
