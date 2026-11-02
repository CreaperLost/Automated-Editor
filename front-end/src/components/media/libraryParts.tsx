import React, { useMemo } from "react";
import { Search, Upload } from "lucide-react";
import { Button, IconButton, Segmented } from "../ui";
import { api } from "../../lib/ipc";
import type { LicenceKind } from "../../lib/types";

/** A file of the user's on its way into a library. */
export interface OwnFile {
  path: string;
  name: string;
  tags: string;
  licence: LicenceKind;
}

/** Asks for a file of the user's; its name, without the folder or extension, starts the form. */
export async function pickOwnFile(): Promise<OwnFile | undefined> {
  const [path] = await api.pickMediaFiles();
  if (!path) return undefined;
  const name = path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "";
  return { path, name, tags: "", licence: "noCredit" };
}

export const splitTags = (tags: string) => tags.split(",").map((t) => t.trim()).filter(Boolean);

/** The items matching what was typed: every word in their id, name or tags. */
export function useLibrarySearch<T extends { id: string; name: string; tags: string[] }>(items: T[], search: string): T[] {
  return useMemo(() => {
    const words = search.trim().toLowerCase().split(/[\s,]+/).filter(Boolean);
    return items.filter((item) => {
      const hay = `${item.id} ${item.name} ${item.tags.join(" ")}`.toLowerCase();
      return words.every((w) => hay.includes(w));
    });
  }, [items, search]);
}

/** The search field, the button adding a file of the user's, and the quick filters. */
export const LibrarySearch: React.FC<{
  noun: string;
  addLabel: string;
  tags: string[];
  search: string;
  setSearch: React.Dispatch<React.SetStateAction<string>>;
  disabled: boolean;
  onAdd: () => void;
}> = ({ noun, addLabel, tags, search, setSearch, disabled, onAdd }) => (
  <div className="shrink-0 space-y-2 p-3 border-b border-studio-800">
    <div className="flex items-center gap-2">
      <div className="relative flex-1">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
        <input
          type="search"
          aria-label={`Search ${noun}`}
          placeholder={`Search ${noun}…`}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="ui-field w-full pl-8"
        />
      </div>
      <IconButton icon={Upload} variant="secondary" label={addLabel} disabled={disabled} onClick={onAdd} />
    </div>
    <div className="flex flex-wrap gap-1">
      {tags.map((tag) => (
        <button
          key={tag}
          type="button"
          onClick={() => setSearch((s) => (s.trim().toLowerCase() === tag ? "" : tag))}
          className={`h-6 px-2 rounded-control border text-meta transition-colors ${
            search.trim().toLowerCase() === tag
              ? "border-audio/60 bg-audio/15 text-audio-fg"
              : "border-studio-700 text-studio-300 hover:bg-studio-850"
          }`}
        >
          {tag}
        </button>
      ))}
    </div>
  </div>
);

/** The form for a file of the user's: its name, tags and licence. */
export const AddOwnFile: React.FC<{
  noun: string;
  tagsHint: string;
  licences: { value: LicenceKind; label: string }[];
  adding: OwnFile;
  setAdding: (next: OwnFile | null) => void;
  /** About the chosen licence, under the choices. */
  warning?: React.ReactNode;
  disabled: boolean;
  onSave: () => void;
}> = ({ noun, tagsHint, licences, adding, setAdding, warning, disabled, onSave }) => (
  <div className="m-3 space-y-2 rounded-panel border border-studio-700 p-3">
    <p className="text-label text-studio-200">Add to your {noun}</p>
    <input aria-label="Name" className="ui-field w-full" value={adding.name} onChange={(e) => setAdding({ ...adding, name: e.target.value })} />
    <input
      aria-label="Tags"
      placeholder={tagsHint}
      className="ui-field w-full"
      value={adding.tags}
      onChange={(e) => setAdding({ ...adding, tags: e.target.value })}
    />
    <Segmented<LicenceKind>
      label="Licence"
      size="sm"
      className="w-full"
      value={adding.licence}
      onChange={(licence) => setAdding({ ...adding, licence })}
      options={licences}
    />
    {warning}
    <div className="flex justify-end gap-2">
      <Button variant="ghost" size="sm" onClick={() => setAdding(null)}>
        Cancel
      </Button>
      <Button variant="primary" size="sm" disabled={disabled} onClick={onSave}>
        Add
      </Button>
    </div>
  </div>
);
