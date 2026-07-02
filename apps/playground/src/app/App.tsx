import type { RefObject } from "preact";
import {
  AlignLeft,
  CodeXml,
  Download,
  FilePlus,
  LogIn,
  Moon,
  PanelLeft,
  PanelRight,
  Pencil,
  Play,
  Share2,
  Split,
  Square,
  Sun,
  Trash2,
  Upload,
  type LucideIcon,
} from "lucide-preact";
import { useEffect, useRef, useState } from "preact/hooks";
import { examples } from "../data/examples";
import { Editor } from "./Editor";
import {
  currentExample,
  currentFile,
  diagnosticLocation,
  type Dispatch,
  type Diagnostic,
  type Model,
} from "../domain/model";

type Props = {
  model: Model;
  dispatch: Dispatch;
};

export function App({ model, dispatch }: Props) {
  const zipInput = useRef<HTMLInputElement>(null);
  const [workspaceOpen, setWorkspaceOpen] = useState(true);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(true);
  const [colorScheme, setColorScheme] = useColorScheme();

  return (
    <main
      class={`playground-shell ${workspaceOpen ? "" : "workspace-collapsed"} ${
        diagnosticsOpen ? "" : "diagnostics-collapsed"
      }`}
    >
      <WorkspacePanel
        model={model}
        dispatch={dispatch}
        zipInput={zipInput}
        open={workspaceOpen}
        toggleOpen={() => setWorkspaceOpen((current) => !current)}
      />

      <section class="editor-panel" aria-label="Loom source editor">
        <EditorToolbar
          model={model}
          dispatch={dispatch}
          workspaceOpen={workspaceOpen}
          diagnosticsOpen={diagnosticsOpen}
          colorScheme={colorScheme}
          toggleWorkspace={() => setWorkspaceOpen((current) => !current)}
          toggleDiagnostics={() => setDiagnosticsOpen((current) => !current)}
          toggleColorScheme={() => {
            setColorScheme((current) => (current === "dark" ? "light" : "dark"));
          }}
        />
        <Editor
          file={currentFile(model)}
          diagnostics={model.diagnostics}
          pendingCursor={model.pendingCursor}
          dispatch={dispatch}
        />
      </section>

      <DiagnosticsPanel
        model={model}
        dispatch={dispatch}
        open={diagnosticsOpen}
        toggleOpen={() => setDiagnosticsOpen((current) => !current)}
      />
    </main>
  );
}

function WorkspacePanel({
  model,
  dispatch,
  zipInput,
  open,
  toggleOpen,
}: {
  model: Model;
  dispatch: Dispatch;
  zipInput: RefObject<HTMLInputElement>;
  open: boolean;
  toggleOpen: () => void;
}) {
  const example = currentExample(model);
  const [contextMenu, setContextMenu] = useState<{ path: string; x: number; y: number } | null>(null);

  useEffect(() => {
    if (!contextMenu) {
      return;
    }

    const close = () => setContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };

    document.addEventListener("click", close);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("click", close);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [contextMenu]);

  return (
    <aside class="workspace-panel" aria-label="Workspace files" aria-hidden={!open}>
      <header class="panel-header">
        <div>
          <p class="eyebrow">Workspace</p>
          <h1>Loom Playground</h1>
        </div>
        <IconButton
          icon="panelLeft"
          label="Collapse workspace"
          onClick={toggleOpen}
        />
      </header>

      <div class="example-picker">
        <label for="example-select">Example</label>
        <select
          id="example-select"
          value={model.currentExampleId}
          onChange={(event) => {
            dispatch({
              type: "load-example-requested",
              exampleId: event.currentTarget.value,
            });
          }}
        >
          {!example && <option value="custom">Custom workspace</option>}
          {examples.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.name}
            </option>
          ))}
        </select>
        <p>{example?.description ?? "Custom workspace"}</p>
      </div>

      <section class="file-section" aria-label="Workspace file management">
        <div class="section-heading">
          <span>Files</span>
          <div class="section-actions">
            <IconButton
              icon="filePlus"
              label="New file"
              disabled={!open}
              onClick={() => dispatch({ type: "new-file-requested" })}
            />
            <button
              type="button"
              class="icon small"
              aria-label="New fragment"
              data-tooltip="New fragment"
              disabled={model.activePath !== model.entryPath}
              onClick={() => dispatch({ type: "fragment-requested" })}
            >
              <Icon name="fragment" />
            </button>
          </div>
        </div>
        <nav class="file-list" aria-label="Files">
          {model.files.map((file) => (
            <button
              key={file.path}
              type="button"
              class={`file-item ${file.path === model.activePath ? "active" : ""}`}
              onClick={() => dispatch({ type: "file-selected", path: file.path })}
              onContextMenu={(event) => {
                event.preventDefault();
                dispatch({ type: "file-selected", path: file.path });
                setContextMenu({ path: file.path, x: event.clientX, y: event.clientY });
              }}
            >
              <span>{file.path}</span>
              {file.path === model.entryPath && <em title="Entry file">entry</em>}
            </button>
          ))}
        </nav>
        {contextMenu && (
          <div
            class="file-context-menu"
            role="menu"
            style={{ left: contextMenu.x, top: contextMenu.y }}
            onClick={(event) => event.stopPropagation()}
          >
            <button
              type="button"
              role="menuitem"
              disabled={contextMenu.path === model.entryPath}
              onClick={() => {
                dispatch({ type: "set-entry-requested" });
                setContextMenu(null);
              }}
            >
              <Icon name="entry" />
              Set Entry
            </button>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                dispatch({ type: "rename-requested" });
                setContextMenu(null);
              }}
            >
              <Icon name="rename" />
              Rename
            </button>
            <button
              type="button"
              role="menuitem"
              disabled={model.files.length <= 1}
              onClick={() => {
                dispatch({ type: "delete-requested" });
                setContextMenu(null);
              }}
            >
              <Icon name="trash" />
              Delete
            </button>
          </div>
        )}
        <div class="file-actions" role="toolbar" aria-label="File actions">
          <IconButton
            icon="entry"
            label="Set entry"
            disabled={model.activePath === model.entryPath}
            onClick={() => dispatch({ type: "set-entry-requested" })}
          />
          <IconButton icon="rename" label="Rename" onClick={() => dispatch({ type: "rename-requested" })} />
          <IconButton
            icon="trash"
            label="Delete"
            disabled={model.files.length <= 1}
            onClick={() => dispatch({ type: "delete-requested" })}
          />
        </div>
      </section>

      <div class="workspace-actions" role="toolbar" aria-label="Workspace actions">
        <IconButton icon="download" label="Export ZIP" onClick={() => dispatch({ type: "export-zip-requested" })} />
        <IconButton icon="upload" label="Import ZIP" onClick={() => zipInput.current?.click()} />
        <input
          ref={zipInput}
          type="file"
          accept=".zip,application/zip"
          hidden
          onChange={(event) => {
            const file = event.currentTarget.files?.[0];
            event.currentTarget.value = "";
            if (file) {
              dispatch({ type: "import-zip-selected", file });
            }
          }}
        />
      </div>
    </aside>
  );
}

function EditorToolbar({
  model,
  dispatch,
  workspaceOpen,
  diagnosticsOpen,
  colorScheme,
  toggleWorkspace,
  toggleDiagnostics,
  toggleColorScheme,
}: Props & {
  workspaceOpen: boolean;
  diagnosticsOpen: boolean;
  colorScheme: ColorScheme;
  toggleWorkspace: () => void;
  toggleDiagnostics: () => void;
  toggleColorScheme: () => void;
}) {
  return (
    <div class="toolbar" role="toolbar" aria-label="Editor and playback actions">
      <IconButton
        icon="panelLeft"
        label={workspaceOpen ? "Collapse workspace" : "Show workspace"}
        onClick={toggleWorkspace}
      />
      <div class="toolbar-group" aria-label="Playback actions">
        <IconButton
          icon="play"
          label="Play"
          disabled={playDisabled(model)}
          onClick={() => dispatch({ type: "play-requested" })}
        />
        <IconButton
          icon="stop"
          label="Stop"
          disabled={!model.isPlaying && !model.isPlaybackLoading}
          onClick={() => dispatch({ type: "stop-requested" })}
        />
      </div>

      <div class="toolbar-group" aria-label="Source actions">
        <IconButton
          icon="compile"
          label="Compile"
          disabled={model.compileStatus === "loading"}
          onClick={() => dispatch({ type: "compile-requested", reason: "manual" })}
        />
        <IconButton
          icon="format"
          label="Format"
          disabled={model.compileStatus === "loading"}
          onClick={() => dispatch({ type: "format-requested" })}
        />
      </div>

      <span class="active-path">
        {model.activePath}
        {model.dirty ? " *" : ""}
      </span>
      <IconButton icon="share" label="Share" onClick={() => dispatch({ type: "share-requested" })} />
      <IconButton
        icon={colorScheme === "dark" ? "sun" : "moon"}
        label={colorScheme === "dark" ? "Use light mode" : "Use dark mode"}
        onClick={toggleColorScheme}
      />
      <IconButton
        icon="panelRight"
        label={diagnosticsOpen ? "Collapse diagnostics" : "Show diagnostics"}
        onClick={toggleDiagnostics}
      />
    </div>
  );
}

function DiagnosticsPanel({
  model,
  dispatch,
  open,
  toggleOpen,
}: Props & {
  open: boolean;
  toggleOpen: () => void;
}) {
  return (
    <aside class="diagnostics-panel" aria-label="Diagnostics" aria-hidden={!open}>
      <header class="panel-header compact">
        <div>
          <p class="eyebrow">Compile</p>
          <h2>Diagnostics</h2>
        </div>
        <div class="panel-header-actions">
          <span class={`status-pill ${statusClass(model)}`}>{statusLabel(model)}</span>
          <IconButton
            icon="panelRight"
            label="Collapse diagnostics"
            onClick={toggleOpen}
          />
        </div>
      </header>
      <Diagnostics diagnostics={model.diagnostics} eventCount={model.eventCount} compileStatus={model.compileStatus} dispatch={dispatch} />
    </aside>
  );
}

function Diagnostics({
  diagnostics,
  eventCount,
  compileStatus,
  dispatch,
}: {
  diagnostics: Diagnostic[];
  eventCount: number;
  compileStatus: Model["compileStatus"];
  dispatch: Dispatch;
}) {
  if (diagnostics.length === 0) {
    return (
      <div class="empty-state">
        {compileStatus === "ok" ? `${eventCount} MIDI events compiled.` : "No diagnostics."}
      </div>
    );
  }

  return (
    <ol class="diagnostics-list">
      {diagnostics.map((diagnostic, index) => (
        <li key={`${diagnosticLocation(diagnostic)}-${index}`}>
          <button type="button" onClick={() => dispatch({ type: "diagnostic-selected", index })}>
            <strong>{diagnosticLocation(diagnostic)}</strong>
            <span>{diagnostic.message}</span>
            {diagnostic.help && <small>{diagnostic.help}</small>}
          </button>
        </li>
      ))}
    </ol>
  );
}

function playDisabled(model: Model): boolean {
  return model.compileStatus === "loading" || model.compileStatus === "err" || model.isPlaying || model.isPlaybackLoading;
}

function statusLabel(model: Model): string {
  if (model.isPlaybackLoading) {
    return "Loading samples";
  }

  if (model.isPlaying) {
    const position = model.playbackPosition;
    if (!position) {
      return "Playing";
    }
    return `Playing ${formatTime(position.seconds)} beat ${position.beat.toFixed(2)}${
      position.loop ? " loop" : ""
    }`;
  }

  switch (model.compileStatus) {
    case "loading":
      return "Loading";
    case "ready":
      return "Ready";
    case "dirty":
      return "Dirty";
    case "ok":
      return "OK";
    case "err":
      return "Error";
  }
}

function statusClass(model: Model): string {
  if (model.isPlaying || model.isPlaybackLoading) {
    return "status-playing";
  }

  return `status-${model.compileStatus}`;
}

function formatTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${minutes}:${remainingSeconds}`;
}

type ColorScheme = "light" | "dark";

const colorSchemeStorageKey = "loom.playground.colorScheme";

function useColorScheme(): [ColorScheme, (update: (current: ColorScheme) => ColorScheme) => void] {
  const [colorScheme, setColorSchemeState] = useState<ColorScheme>(readInitialColorScheme);

  useEffect(() => {
    document.documentElement.style.colorScheme = colorScheme;
    localStorage.setItem(colorSchemeStorageKey, colorScheme);
  }, [colorScheme]);

  return [colorScheme, setColorSchemeState];
}

function readInitialColorScheme(): ColorScheme {
  const stored = localStorage.getItem(colorSchemeStorageKey);
  if (stored === "light" || stored === "dark") {
    return stored;
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function IconButton({
  icon,
  label,
  disabled,
  onClick,
}: {
  icon: IconName;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      class="icon small"
      aria-label={label}
      data-tooltip={label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} />
    </button>
  );
}

type IconName =
  | "compile"
  | "download"
  | "entry"
  | "filePlus"
  | "format"
  | "fragment"
  | "panelLeft"
  | "panelRight"
  | "play"
  | "rename"
  | "share"
  | "moon"
  | "stop"
  | "sun"
  | "trash"
  | "upload";

function Icon({ name }: { name: IconName }) {
  const Component = icons[name];
  return <Component aria-hidden="true" size={16} strokeWidth={2} />;
}

const icons = {
  compile: CodeXml,
  download: Download,
  entry: LogIn,
  filePlus: FilePlus,
  format: AlignLeft,
  fragment: Split,
  panelLeft: PanelLeft,
  panelRight: PanelRight,
  play: Play,
  rename: Pencil,
  share: Share2,
  moon: Moon,
  stop: Square,
  sun: Sun,
  trash: Trash2,
  upload: Upload,
} satisfies Record<IconName, LucideIcon>;
