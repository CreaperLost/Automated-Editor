import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Bot, Check, Copy, Download, Link2, Link2Off, Trash2, X } from "lucide-react";
import { api } from "../../lib/ipc";
import type { McpApp, McpAppState, McpSkillHome, McpSkillState, McpStatus } from "../../lib/types";
import { Button, Switch, Tabs, cn } from "../ui";

const APP_NAMES: Record<McpApp, { name: string; restart: string }> = {
  claudeApp: { name: "Claude app", restart: "Restart the Claude app to load it." },
  codex: { name: "Codex", restart: "Restart Codex (app or CLI) to load it." },
};

const SKILL_HOMES: Record<McpSkillHome, string> = {
  claudeCode: "Claude Code (and the Claude app's Code tab)",
  codex: "Codex",
};

type Other = "claudeCode" | "cli" | "gemini" | "cursor" | "other";

const OTHERS: { value: Other; label: string }[] = [
  { value: "claudeCode", label: "Claude Code" },
  { value: "cli", label: "Command line" },
  { value: "gemini", label: "Gemini CLI" },
  { value: "cursor", label: "Cursor" },
  { value: "other", label: "Other" },
];

/** What to set up in an agent the app cannot connect for you: it starts `<exe> --mcp-stdio`. */
function setup(client: Other, exe: string): { where: string; text: string } {
  const server = { command: exe, args: ["--mcp-stdio"] };
  switch (client) {
    case "claudeCode":
      return {
        where: "For the Claude Code CLI and the Code tab of the Claude app. Run in a terminal:",
        text: `claude mcp add --scope user aeroedits -- "${exe}" --mcp-stdio`,
      };
    case "cli":
      return {
        where:
          "Agents with a shell (Claude Code, Codex) can drive AeroEdits from the command line with nothing to set up, which keeps their context small. Add this to the agent's notes (CLAUDE.md or AGENTS.md):",
        text: [
          `AeroEdits (the video editor) is driven with: "${exe}" do <action> [--key value]`,
          `List the actions: "${exe}" do. Options of one: "${exe}" do <action> --help.`,
          "For an edit, start with `do template tutorial`: the steps, what to ask me, and how to export. Each action is one undo step and answers in a line or two.",
        ].join("\n"),
      };
    case "gemini":
      return {
        where: "Add to ~/.gemini/settings.json:",
        text: JSON.stringify({ mcpServers: { aeroedits: server } }, null, 2),
      };
    case "cursor":
      return {
        where: "Add to ~/.cursor/mcp.json:",
        text: JSON.stringify({ mcpServers: { aeroedits: server } }, null, 2),
      };
    case "other":
      return {
        where: "Any MCP client that can start a local program (stdio):",
        text: `Command  ${exe}\nArgs     --mcp-stdio`,
      };
  }
}

const CopyButton: React.FC<{ text: string; label: string }> = ({ text, label }) => {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={() => {
        void navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
    >
      {copied ? <Check className="w-3.5 h-3.5 text-accent-fg" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  );
};

const AppRow: React.FC<{
  state: McpAppState;
  busy: boolean;
  justConnected: boolean;
  onConnect: (connected: boolean) => void;
}> = ({ state, busy, justConnected, onConnect }) => {
  const { name, restart } = APP_NAMES[state.app];
  const note = !state.installed
    ? "Not installed"
    : state.otherCopy
      ? "Set up for another copy of AeroEdits"
      : state.connected
        ? justConnected
          ? `Connected. ${restart}`
          : "Connected"
        : "Not connected";
  return (
    <div className="flex items-center gap-3 px-3 py-2">
      <div className="flex-1 min-w-0">
        <div className="text-xs text-studio-200">{name}</div>
        <div
          className={cn("text-meta truncate", state.connected && !state.otherCopy ? "text-accent-fg" : "text-studio-500")}
          title={state.configPath}
        >
          {note}
        </div>
      </div>
      {state.connected && !state.otherCopy ? (
        <Button size="sm" variant="ghost" icon={Link2Off} disabled={busy} onClick={() => onConnect(false)}>
          Disconnect
        </Button>
      ) : (
        <Button
          size="sm"
          variant="subtle"
          icon={Link2}
          disabled={busy || !state.installed}
          onClick={() => onConnect(true)}
          title={`Adds AeroEdits to ${state.configPath} (the old file is backed up)`}
        >
          {state.otherCopy ? "Use this copy" : "Connect"}
        </Button>
      )}
    </div>
  );
};

const SkillRow: React.FC<{
  state: McpSkillState;
  busy: boolean;
  onInstall: (installed: boolean) => void;
}> = ({ state, busy, onInstall }) => {
  const note = !state.available
    ? "Not installed"
    : !state.installed
      ? "Skill not installed"
      : state.current
        ? "Installed"
        : "Installed, older or changed copy";
  return (
    <div className="flex items-center gap-3 px-3 py-2">
      <div className="flex-1 min-w-0">
        <div className="text-xs text-studio-200">{SKILL_HOMES[state.home]}</div>
        <div
          className={cn("text-meta truncate", state.installed ? "text-accent-fg" : "text-studio-500")}
          title={state.path}
        >
          {note}
        </div>
      </div>
      {state.installed && (
        <Button size="sm" variant="ghost" icon={Trash2} disabled={busy} onClick={() => onInstall(false)}>
          Remove
        </Button>
      )}
      {(!state.installed || !state.current) && (
        <Button
          size="sm"
          variant="subtle"
          icon={Download}
          disabled={busy || !state.available}
          onClick={() => onInstall(true)}
          title={`Writes the skill to ${state.path}`}
        >
          {state.installed ? "Update" : "Install"}
        </Button>
      )}
    </div>
  );
};

/// AI agents: lets an agent the user already runs (the Claude app, Codex, ...) work on the open
/// project. The agent starts AeroEdits' bridge, which reaches this window over a private pipe:
/// nothing listens on the network and only this computer account can connect.
export const AgentsDialog: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [status, setStatus] = useState<McpStatus>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [other, setOther] = useState<Other>("claudeCode");
  const [justConnected, setJustConnected] = useState<McpApp[]>([]);

  useEffect(() => {
    let active = true;
    const refresh = () =>
      api
        .mcpStatus()
        .then((next) => active && setStatus(next))
        .catch((e) => active && setError(String(e)));
    void refresh();
    // Connected agents and their calls follow along while the dialog is open.
    const timer = window.setInterval(refresh, 2000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const act = (call: () => Promise<McpStatus>, after?: () => void) => {
    setBusy(true);
    setError(undefined);
    void call()
      .then((next) => {
        setStatus(next);
        after?.();
      })
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(false));
  };

  const exe = status?.bridgeCommand ?? "aeroedits.exe";
  const snippet = setup(other, exe);
  const connected = status?.connected ?? [];
  const summary = !status?.enabled
    ? "Off"
    : !status.running
      ? "Not running"
      : connected.length === 0
        ? "Ready, no agent connected"
        : `Connected: ${connected.join(", ")}`;

  return createPortal(
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="AI agents"
        className="w-full max-w-2xl max-h-[85vh] flex flex-col rounded-panel border border-studio-700 bg-studio-900 shadow-dialog"
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-studio-800">
          <div className="flex items-center gap-2 text-heading text-studio-100">
            <Bot className="w-4 h-4 text-studio-400" />
            AI agents
          </div>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-4">
          <p className="text-xs text-studio-300 leading-relaxed">
            Let an AI agent you already use work on the open project: read the transcript, look at
            frames, and propose cuts and timeline edits. Its proposals wait in the Agent panel until
            you accept them, each batch as one undo step. It runs on your own plan.
            The connection stays on this computer: nothing listens on the network, and only your
            account here can connect.
          </p>

          <div className="flex items-center justify-between gap-3 rounded-control border border-studio-800 bg-studio-850 px-3 py-2">
            <Switch
              checked={!!status?.enabled}
              disabled={!status || busy}
              onChange={(enabled) => act(() => api.mcpSetEnabled(enabled))}
              label="Allow agents to connect"
            />
            <span
              className={cn(
                "text-meta truncate",
                status?.running ? "text-accent-fg" : status?.error ? "text-danger-fg" : "text-studio-500",
              )}
              title={status?.endpoint ?? undefined}
            >
              {summary}
            </span>
          </div>
          {(error || status?.error) && <p className="text-meta text-danger-fg">{error ?? status?.error}</p>}

          {status?.enabled && (
            <>
              <section className="space-y-1.5">
                <h3 className="text-label font-semibold text-studio-300">Connect your agent</h3>
                <div className="divide-y divide-studio-800/60 rounded-control border border-studio-800">
                  {status.apps.map((state) => (
                    <AppRow
                      key={state.app}
                      state={state}
                      busy={busy}
                      justConnected={justConnected.includes(state.app)}
                      onConnect={(on) =>
                        act(
                          () => api.mcpAppConnect(state.app, on),
                          () =>
                            setJustConnected((list) =>
                              on ? [...list, state.app] : list.filter((a) => a !== state.app),
                            ),
                        )
                      }
                    />
                  ))}
                </div>
                <p className="text-meta text-studio-400">
                  Your agent app may ask you to allow each AeroEdits action the first time. That
                  question is the agent app's, not AeroEdits'. Only transcribing, the filler and
                  retake review, and installing the graphics renderer go online; they are marked
                  so, so you can allow them once.
                </p>
              </section>

              <section className="space-y-1.5">
                <h3 className="text-label font-semibold text-studio-300">Editing skill</h3>
                <p className="text-meta text-studio-400">
                  Agents get AeroEdits' editing templates (a tutorial edit: what to ask you, the
                  order of the passes, how to export) from the app on their own. Agents that read
                  skills from disk also pick them up when you ask for an edit; the app keeps the
                  installed copy up to date.
                </p>
                <div className="divide-y divide-studio-800/60 rounded-control border border-studio-800">
                  {(status.skills ?? []).map((state) => (
                    <SkillRow
                      key={state.home}
                      state={state}
                      busy={busy}
                      onInstall={(on) => act(() => api.mcpSkillInstall(state.home, on))}
                    />
                  ))}
                </div>
              </section>

              <section className="space-y-1.5">
                <h3 className="text-label font-semibold text-studio-300">Other agents</h3>
                <Tabs items={OTHERS} value={other} onChange={setOther} label="Agent" size="sm" />
                <p className="text-meta text-studio-400">{snippet.where}</p>
                <div className="relative rounded-control border border-studio-800 bg-studio-950">
                  <pre className="overflow-x-auto px-3 py-2 pr-9 font-mono text-meta text-studio-200 whitespace-pre-wrap break-all">
                    {snippet.text}
                  </pre>
                  <div className="absolute top-1 right-1">
                    <CopyButton text={snippet.text} label="Copy" />
                  </div>
                </div>
              </section>

              <section className="space-y-1.5">
                <h3 className="text-label font-semibold text-studio-300">What agents did</h3>
                {status.activity.length === 0 ? (
                  <p className="text-meta text-studio-500">Nothing yet.</p>
                ) : (
                  <ul className="divide-y divide-studio-800/60 rounded-control border border-studio-800">
                    {status.activity.map((a, i) => (
                      <li key={`${a.at}-${i}`} className="flex items-center gap-3 px-3 py-1 text-meta">
                        <span className="w-16 shrink-0 font-mono text-studio-500">{a.at}</span>
                        <span className="w-20 shrink-0 truncate text-studio-400" title={a.client ?? undefined}>
                          {a.client ?? "agent"}
                        </span>
                        <span className={cn("w-32 shrink-0 font-mono", a.ok ? "text-studio-200" : "text-danger-fg")}>
                          {a.tool}
                        </span>
                        <span className="flex-1 min-w-0 truncate text-studio-500" title={a.error}>
                          {a.ok ? `${a.millis} ms` : a.error}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
};
