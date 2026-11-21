import React, { useEffect, useMemo, useState } from "react";
import { Check, ChevronDown, ChevronRight, Copy, Search } from "lucide-react";
import { Badge, Button, Notice, cn } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { ImageLibraryView, ImageTopic, LibraryImage, Licence } from "../../lib/types";

const SOURCED = "sourced";
/** Pictures asked for at once (the command takes at most 60). */
const BATCH = 60;

const needsNoCredit = (licence: Licence) => ["cc0", "publicDomain", "noCredit"].includes(licence.kind);
/** A picture's group: its id without its name. */
const groupOf = (image: LibraryImage) => image.id.slice(0, image.id.lastIndexOf("/"));
const nameOf = (image: LibraryImage) => image.id.slice(image.id.lastIndexOf("/") + 1).replace(/-/g, " ");

/// The built-in image library (FCL-5): the pictures graphics use, one set per look (a video
/// keeps one) plus sourced flags, maps and wordmarks shown with every look. Pick a picture to
/// see what it shows, where it goes and its licence, and copy its id for your agent, which puts
/// it into a graphic.
export const ImagesPage: React.FC = () => {
  const projectHandle = useProjectStore((s) => s.openedProject?.projectHandle);
  const [view, setView] = useState<ImageLibraryView | null>(null);
  const [look, setLook] = useState<string>("");
  const [search, setSearch] = useState("");
  const [found, setFound] = useState<string[] | null>(null);
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  const [pictures, setPictures] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    let active = true;
    api
      .imageLibraryList()
      .then((v) => {
        if (!active) return;
        setView(v);
        const first = v.projectLook ?? v.catalog.looks.find((l) => l.id !== SOURCED)?.id ?? "";
        setLook(first);
        const topic = v.catalog.topics.find((t) => t.look === first);
        setOpenGroup(topic?.groups[0] ? `${topic.id}/${topic.groups[0].id}` : null);
      })
      .catch((err) => active && setError(String(err)));
    return () => {
      active = false;
    };
  }, [projectHandle]);

  const images = useMemo(() => new Map((view?.catalog.images ?? []).map((i) => [i.id, i])), [view]);
  const topics = useMemo<ImageTopic[]>(
    // The look's own first, then the sourced ones shown with every look.
    () => [
      ...(view?.catalog.topics ?? []).filter((t) => t.look === look),
      ...(view?.catalog.topics ?? []).filter((t) => t.look === SOURCED),
    ],
    [view, look],
  );

  // What a search finds, a moment after typing stops.
  useEffect(() => {
    const query = search.trim();
    if (!query) {
      setFound(null);
      return;
    }
    let active = true;
    const timer = window.setTimeout(() => {
      api
        .imageLibrarySearch(query, look || null)
        .then((ids) => active && setFound(ids))
        .catch((err) => active && setError(String(err)));
    }, 250);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [search, look]);

  const shown: LibraryImage[] = useMemo(() => {
    if (found) return found.map((id) => images.get(id)).filter((i): i is LibraryImage => !!i);
    if (!openGroup) return [];
    return (view?.catalog.images ?? []).filter((i) => groupOf(i) === openGroup);
  }, [found, openGroup, images, view]);

  // Fetch the pictures on screen that haven't come yet.
  useEffect(() => {
    const missing = shown.map((i) => i.id).filter((id) => !(id in pictures));
    if (missing.length === 0) return;
    let active = true;
    void (async () => {
      for (let at = 0; at < missing.length && active; at += BATCH) {
        try {
          const got = await api.imageLibraryPictures(missing.slice(at, at + BATCH));
          if (!active) return;
          setPictures((had) => ({ ...had, ...Object.fromEntries(got.map((p) => [p.id, p.src])) }));
        } catch (err) {
          if (active) setError(String(err));
          return;
        }
      }
    })();
    return () => {
      active = false;
    };
  }, [shown]);

  const ground = view?.catalog.looks.find((l) => l.id === look)?.ground || undefined;
  const picked = selected ? images.get(selected) : undefined;

  const copy = async (id: string) => {
    try {
      await navigator.clipboard.writeText(id);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch (err) {
      setError(`Couldn't copy: ${String(err)}`);
    }
  };

  const grid = (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(88px,1fr))] gap-1.5" role="list" aria-label="Pictures">
      {shown.map((image) => (
        <button
          key={image.id}
          type="button"
          role="listitem"
          title={image.depicts}
          aria-label={nameOf(image)}
          aria-pressed={selected === image.id}
          onClick={() => {
            setSelected(selected === image.id ? null : image.id);
            setCopied(false);
          }}
          className={cn(
            "relative aspect-square overflow-hidden rounded-control border",
            selected === image.id ? "border-accent ring-1 ring-accent" : "border-studio-700 hover:border-studio-500",
          )}
          style={{ background: image.look === SOURCED ? "#ffffff" : ground }}
        >
          {pictures[image.id] ? (
            <img src={pictures[image.id]} alt="" className="h-full w-full object-contain p-1" draggable={false} />
          ) : (
            <span className="absolute inset-0 animate-pulse bg-studio-800/40" />
          )}
          {!needsNoCredit(image.licence) && (
            <span className="absolute right-1 top-1 rounded bg-suggest/90 px-1 text-[10px] font-semibold text-black">*</span>
          )}
        </button>
      ))}
    </div>
  );

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="shrink-0 space-y-2 p-3 border-b border-studio-800">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
          <input
            type="search"
            aria-label="Search pictures"
            placeholder="Search pictures…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="ui-field w-full pl-8"
          />
        </div>
        <label className="flex items-center gap-2 text-meta text-studio-400">
          Look
          <select
            className="ui-field flex-1"
            value={look}
            onChange={(e) => {
              setLook(e.target.value);
              setOpenGroup(null);
              setSelected(null);
            }}
          >
            {(view?.catalog.looks ?? [])
              .filter((l) => l.id !== SOURCED)
              .map((l) => (
                <option key={l.id} value={l.id}>
                  {l.id}
                  {l.id === view?.projectLook ? " (this project's)" : ""}
                </option>
              ))}
          </select>
        </label>
      </div>
      {error && (
        <Notice tone="danger" className="mx-3 mt-3 rounded-control border" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {found ? (
          found.length === 0 ? (
            <p className="px-2 py-3 text-label text-studio-500">Nothing matches.</p>
          ) : (
            grid
          )
        ) : (
          topics.map((topic) => (
            <section key={topic.id} aria-label={topic.description}>
              {topic.id !== topic.look && (
                <p className="px-1 pt-2 pb-1 text-meta font-semibold uppercase tracking-wide text-studio-500">
                  {topic.look === SOURCED ? `Sourced: ${topic.description}` : topic.description}
                </p>
              )}
              {topic.groups.map((group) => {
                const key = `${topic.id}/${group.id}`;
                const open = openGroup === key;
                return (
                  <div key={key}>
                    <button
                      type="button"
                      aria-expanded={open}
                      onClick={() => setOpenGroup(open ? null : key)}
                      className="flex w-full items-center gap-1.5 rounded-control px-1.5 py-1 text-left hover:bg-studio-850"
                    >
                      {open ? <ChevronDown className="w-3.5 h-3.5 shrink-0" /> : <ChevronRight className="w-3.5 h-3.5 shrink-0" />}
                      <span className="min-w-0 flex-1 truncate text-label text-studio-100">{group.description}</span>
                      {group.sourced && <Badge tone="suggest">Sourced</Badge>}
                      <span className="text-meta text-studio-500 tabular-nums">{group.count}</span>
                    </button>
                    {open && <div className="px-1 pb-2 pt-1">{grid}</div>}
                  </div>
                );
              })}
            </section>
          ))
        )}
      </div>
      {picked && (
        <div className="shrink-0 space-y-1 border-t border-studio-800 p-3 text-meta" aria-label="Picture">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="truncate text-label font-medium text-studio-100">{nameOf(picked)}</span>
            {picked.width && picked.height && (
              <span className="text-studio-500 tabular-nums">
                {picked.width}×{picked.height}
              </span>
            )}
            {!needsNoCredit(picked.licence) && <Badge tone="suggest">{picked.licence.kind === "unknown" ? "Check rights" : "Needs credit"}</Badge>}
          </div>
          <p className="text-studio-300">{picked.depicts}</p>
          <p className="text-studio-500">{picked.placement}</p>
          {!needsNoCredit(picked.licence) && (
            <p className="text-suggest-fg">
              {picked.rights ?? "Its licence isn't known."} Check flags it before export.
            </p>
          )}
          {picked.look !== SOURCED && view?.projectLook && picked.look !== view.projectLook && (
            <p className="text-suggest-fg">Drawn for the {picked.look} look, not this project's: Check flags it.</p>
          )}
          <div className="flex items-center gap-2 pt-1">
            <Button size="sm" variant="secondary" icon={copied ? Check : Copy} onClick={() => void copy(picked.id)}>
              {copied ? "Copied" : "Copy id"}
            </Button>
            <span className="min-w-0 truncate text-studio-500">Give your agent the id to put it in a graphic.</span>
          </div>
        </div>
      )}
    </div>
  );
};
