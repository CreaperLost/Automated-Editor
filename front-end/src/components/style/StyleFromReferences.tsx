import React, { useEffect, useState } from "react";
import { Check, Copy, FilePlus2, X } from "lucide-react";
import { api } from "../../lib/ipc";
import type { StyleRequest } from "../../lib/types";
import { Button, IconButton } from "../ui";

const MAX_REFERENCES = 8;

/// A new style from references (STY-3): the user picks pictures or clips and says what they
/// like; the app keeps the request for their agent (its `status` names it) and copies it as a
/// prompt to paste. The agent does the looking, so nothing here spends the app's AI key.
export const StyleFromReferences: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [references, setReferences] = useState<string[]>([]);
  const [brief, setBrief] = useState("");
  const [name, setName] = useState("");
  const [waiting, setWaiting] = useState<StyleRequest | null>(null);
  const [prompt, setPrompt] = useState<string>();
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    void api
      .styleRequestGet()
      .then(setWaiting)
      .catch(() => undefined);
  }, []);

  const add = async () => {
    try {
      const picked = await api.pickMediaFiles();
      setReferences((now) => [...now, ...picked.filter((p) => !now.includes(p))].slice(0, MAX_REFERENCES));
    } catch (err) {
      setError(String(err));
    }
  };
  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch {
      setError("Could not copy to the clipboard: select the text and copy it.");
    }
  };
  const ask = async () => {
    setError(undefined);
    try {
      const text = await api.styleRequestSet(references, brief, name.trim() || undefined);
      setPrompt(text);
      setWaiting(await api.styleRequestGet());
      await copy(text);
    } catch (err) {
      setError(String(err));
    }
  };
  const drop = async () => {
    await api.styleRequestClear().catch(() => undefined);
    setWaiting(null);
    setPrompt(undefined);
  };
  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        role="dialog"
        aria-modal="true"
        aria-label="New style from references"
        className="w-full max-w-lg flex flex-col gap-3 rounded-panel border border-studio-700 bg-studio-900 shadow-dialog p-4"
        onSubmit={(event) => {
          event.preventDefault();
          void ask();
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
        }}
      >
        <h2 className="text-body font-semibold text-studio-100">New style from references</h2>
        {prompt ? (
          <>
            <p className="text-label text-studio-400 leading-relaxed">
              {copied ? "Copied. " : ""}Paste it to Claude Code or Codex. An agent that is already connected sees it too
              when it next checks AeroEdits (its status names the request), so &ldquo;make the style I asked for&rdquo;
              is enough. It shows you stills and a clip before it saves anything.
            </p>
            <textarea readOnly value={prompt} aria-label="The prompt" className="ui-field font-mono text-meta h-40 resize-none" />
            <div className="flex justify-end gap-2">
              <Button type="button" variant="ghost" icon={copied ? Check : Copy} onClick={() => void copy(prompt)}>
                Copy again
              </Button>
              <Button type="button" variant="primary" onClick={onClose}>
                Done
              </Button>
            </div>
          </>
        ) : (
          <>
            <p className="text-label text-studio-400 leading-relaxed">
              Pick pictures or clips whose look you want (up to {MAX_REFERENCES}), and say what you like about them. Your
              agent breaks them down into a style; the app sends nothing anywhere.
            </p>
            {waiting && (
              <div className="flex items-center gap-2 rounded border border-studio-700 p-2 text-meta text-studio-400">
                <span className="flex-1">
                  A request is waiting for your agent ({waiting.references.length} reference
                  {waiting.references.length === 1 ? "" : "s"}): a new one replaces it.
                </span>
                <Button type="button" size="sm" variant="ghost" onClick={() => void drop()}>
                  Drop it
                </Button>
              </div>
            )}
            <ul className="flex flex-col gap-1">
              {references.map((path) => (
                <li key={path} className="flex items-center gap-2 text-label text-studio-300" title={path}>
                  <span className="flex-1 truncate">{fileName(path)}</span>
                  <IconButton
                    icon={X}
                    size="sm"
                    label="Leave it out"
                    onClick={() => setReferences((now) => now.filter((p) => p !== path))}
                  />
                </li>
              ))}
            </ul>
            <Button
              type="button"
              size="sm"
              variant="ghost"
              icon={FilePlus2}
              disabled={references.length >= MAX_REFERENCES}
              onClick={() => void add()}
            >
              Add pictures or clips…
            </Button>
            <textarea
              aria-label="What you like about them"
              placeholder="What you like, e.g. the big two-word captions and the cards that slide up from the bottom"
              value={brief}
              maxLength={1000}
              onChange={(e) => setBrief(e.target.value)}
              className="ui-field h-20 resize-none"
            />
            <input
              aria-label="The style's name"
              placeholder="Name (optional), e.g. Bold shorts"
              value={name}
              maxLength={40}
              onChange={(e) => setName(e.target.value)}
              className="ui-field"
            />
            {error && (
              <p role="alert" className="text-meta text-danger-fg">
                {error}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <Button type="button" variant="ghost" onClick={onClose}>
                Cancel
              </Button>
              <Button type="submit" variant="primary" disabled={references.length === 0 || !brief.trim()}>
                Ask my agent
              </Button>
            </div>
          </>
        )}
      </form>
    </div>
  );
};
