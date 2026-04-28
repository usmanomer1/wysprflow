import { useEffect, useState } from "react";
import { Plus, Star, StarOff, Trash2, Sparkles, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { tauri, type DictEntry } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function DictionarySection() {
  const [entries, setEntries] = useState<DictEntry[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [loaded, setLoaded] = useState(false);

  const refresh = async () => {
    try {
      const list = await tauri.listDictionary();
      setEntries(list);
    } catch (e) {
      toast.error(`Couldn't load dictionary: ${e}`);
    } finally {
      setLoaded(true);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const onAdd = async () => {
    const word = draft.trim();
    if (!word) return;
    setBusy(true);
    try {
      await tauri.addDictionaryWord(word);
      setDraft("");
      await refresh();
    } catch (e) {
      toast.error(`Failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const onDelete = async (id: number) => {
    try {
      await tauri.deleteDictionaryWord(id);
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
    }
  };

  const onToggleStar = async (id: number) => {
    try {
      const updated = await tauri.toggleDictionaryStar(id);
      setEntries((prev) =>
        prev
          .map((e) => (e.id === id ? updated : e))
          .sort((a, b) => {
            if (a.isStarred !== b.isStarred) return a.isStarred ? -1 : 1;
            return a.updatedAt > b.updatedAt ? -1 : 1;
          }),
      );
    } catch (e) {
      toast.error(`Star failed: ${e}`);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <Input
          placeholder="Add a name, acronym, or jargon word…"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onAdd()}
          className="font-mono"
        />
        <Button onClick={onAdd} disabled={!draft.trim() || busy}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : <Plus className="size-4" />}
        </Button>
      </div>

      {!loaded ? (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="size-3 animate-spin" /> Loading…
        </div>
      ) : entries.length === 0 ? (
        <div className="rounded border border-dashed border-border/60 bg-muted/20 p-6 text-center">
          <p className="text-xs text-muted-foreground">
            No words yet. Add names, acronyms, or jargon — they'll be passed to the cleanup prompt
            so Haiku spells them right.
          </p>
        </div>
      ) : (
        <ul className="space-y-1">
          {entries.map((e) => (
            <li
              key={e.id}
              className="flex items-center gap-2 rounded-md border border-border/40 bg-card/30 px-3 py-1.5 transition-colors hover:bg-card/60"
            >
              <button
                onClick={() => onToggleStar(e.id)}
                className="text-muted-foreground transition-colors hover:text-yellow-500"
                title={e.isStarred ? "Unstar" : "Star"}
              >
                {e.isStarred ? (
                  <Star className="size-3.5 fill-yellow-500 text-yellow-500" />
                ) : (
                  <StarOff className="size-3.5" />
                )}
              </button>
              <span className="flex-1 font-mono text-sm">{e.word}</span>
              {e.autoLearned ? (
                <Sparkles className="size-3 text-purple-400" aria-label="Auto-learned" />
              ) : null}
              {e.usageCount > 0 ? (
                <span className="text-[10px] text-muted-foreground">×{e.usageCount}</span>
              ) : null}
              <Button size="sm" variant="ghost" onClick={() => onDelete(e.id)}>
                <Trash2 className="size-3" />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
