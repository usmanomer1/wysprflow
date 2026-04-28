import { useEffect, useState } from "react";
import { Plus, Trash2, Pencil, X, Check, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { tauri, type Snippet } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";

export function SnippetsSection() {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [trigger, setTrigger] = useState("");
  const [expansion, setExpansion] = useState("");
  const [busy, setBusy] = useState(false);
  const [showForm, setShowForm] = useState(false);

  const refresh = async () => {
    try {
      const list = await tauri.listSnippets();
      setSnippets(list);
    } catch (e) {
      toast.error(`Couldn't load snippets: ${e}`);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const startNew = () => {
    setEditingId(null);
    setTrigger("");
    setExpansion("");
    setShowForm(true);
  };

  const startEdit = (s: Snippet) => {
    setEditingId(s.id);
    setTrigger(s.trigger);
    setExpansion(s.expansion);
    setShowForm(true);
  };

  const cancel = () => {
    setShowForm(false);
    setEditingId(null);
    setTrigger("");
    setExpansion("");
  };

  const save = async () => {
    if (!trigger.trim() || !expansion.trim()) return;
    setBusy(true);
    try {
      await tauri.upsertSnippet(editingId, trigger.trim(), expansion);
      cancel();
      await refresh();
    } catch (e) {
      toast.error(`Save failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const onDelete = async (id: number) => {
    try {
      await tauri.deleteSnippet(id);
      setSnippets((prev) => prev.filter((s) => s.id !== id));
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
    }
  };

  return (
    <div className="space-y-3">
      {showForm ? (
        <div className="space-y-3 rounded-md border border-border bg-card/40 p-3">
          <div className="space-y-1">
            <Label className="text-xs">Trigger phrase (what you say)</Label>
            <Input
              placeholder="e.g. address, signature, debugging prompt"
              value={trigger}
              onChange={(e) => setTrigger(e.target.value)}
              autoFocus
            />
          </div>
          <div className="space-y-1">
            <Label className="text-xs">Expansion (what gets pasted)</Label>
            <Textarea
              rows={4}
              value={expansion}
              onChange={(e) => setExpansion(e.target.value)}
              className="font-mono"
              placeholder="The exact text to paste when you say the trigger phrase."
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={cancel} disabled={busy}>
              <X className="size-3.5" /> Cancel
            </Button>
            <Button onClick={save} disabled={!trigger.trim() || !expansion.trim() || busy}>
              {busy ? <Loader2 className="size-3.5 animate-spin" /> : <Check className="size-3.5" />}
              {editingId ? "Update" : "Save"}
            </Button>
          </div>
        </div>
      ) : (
        <Button variant="outline" size="sm" onClick={startNew}>
          <Plus className="size-3.5" /> New snippet
        </Button>
      )}

      {snippets.length === 0 ? (
        <div className="rounded border border-dashed border-border/60 bg-muted/20 p-6 text-center">
          <p className="text-xs text-muted-foreground">
            No snippets yet. Snippets bypass cleanup — say a trigger and the expansion pastes
            verbatim. Great for addresses, signatures, scheduling links, FAQs.
          </p>
        </div>
      ) : (
        <ul className="space-y-1.5">
          {snippets.map((s) => (
            <li
              key={s.id}
              className="rounded-md border border-border/40 bg-card/30 px-3 py-2 transition-colors hover:bg-card/60"
            >
              <div className="flex items-start gap-2">
                <div className="flex-1 space-y-0.5 overflow-hidden">
                  <div className="font-mono text-sm font-medium">{s.trigger}</div>
                  <div className="line-clamp-2 text-xs text-muted-foreground">{s.expansion}</div>
                </div>
                <div className="flex gap-1">
                  <Button size="sm" variant="ghost" onClick={() => startEdit(s)}>
                    <Pencil className="size-3" />
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => onDelete(s.id)}>
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
