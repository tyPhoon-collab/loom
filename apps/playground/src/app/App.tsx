import type { RefObject } from "preact";
import { useRef } from "preact/hooks";
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

  return (
    <main class="playground-shell">
      <WorkspacePanel model={model} dispatch={dispatch} zipInput={zipInput} />

      <section class="editor-panel" aria-label="Loom source editor">
        <EditorToolbar model={model} dispatch={dispatch} />
        <Editor
          file={currentFile(model)}
          diagnostics={model.diagnostics}
          pendingCursor={model.pendingCursor}
          dispatch={dispatch}
        />
      </section>

      <DiagnosticsPanel model={model} dispatch={dispatch} />
    </main>
  );
}

function WorkspacePanel({
  model,
  dispatch,
  zipInput,
}: {
  model: Model;
  dispatch: Dispatch;
  zipInput: RefObject<HTMLInputElement>;
}) {
  const example = currentExample(model);

  return (
    <aside class="workspace-panel" aria-label="Workspace files">
      <header class="panel-header">
        <div>
          <p class="eyebrow">Workspace</p>
          <h1>Loom Playground</h1>
        </div>
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
            <button type="button" class="small" onClick={() => dispatch({ type: "new-file-requested" })}>
              New File
            </button>
            <button
              type="button"
              class="small"
              disabled={model.activePath !== model.entryPath}
              onClick={() => dispatch({ type: "fragment-requested" })}
            >
              New Fragment
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
            >
              <span>{file.path}</span>
              {file.path === model.entryPath && <em title="Entry file">entry</em>}
            </button>
          ))}
        </nav>
        <div class="file-actions" role="toolbar" aria-label="File actions">
          <button
            type="button"
            disabled={model.activePath === model.entryPath}
            onClick={() => dispatch({ type: "set-entry-requested" })}
          >
            Set Entry
          </button>
          <button type="button" onClick={() => dispatch({ type: "rename-requested" })}>
            Rename
          </button>
          <button type="button" disabled={model.files.length <= 1} onClick={() => dispatch({ type: "delete-requested" })}>
            Delete
          </button>
        </div>
      </section>

      <div class="workspace-actions" role="toolbar" aria-label="Workspace actions">
        <button type="button" onClick={() => dispatch({ type: "export-zip-requested" })}>
          Export ZIP
        </button>
        <button type="button" onClick={() => zipInput.current?.click()}>
          Import ZIP
        </button>
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

function EditorToolbar({ model, dispatch }: Props) {
  return (
    <div class="toolbar" role="toolbar" aria-label="Editor and playback actions">
      <div class="toolbar-group" aria-label="Playback actions">
        <button type="button" disabled={playDisabled(model)} onClick={() => dispatch({ type: "play-requested" })}>
          Play
        </button>
        <button
          type="button"
          disabled={!model.isPlaying && !model.isPlaybackLoading}
          onClick={() => dispatch({ type: "stop-requested" })}
        >
          Stop
        </button>
      </div>

      <div class="toolbar-group" aria-label="Source actions">
        <button
          type="button"
          disabled={model.compileStatus === "loading"}
          onClick={() => dispatch({ type: "compile-requested", reason: "manual" })}
        >
          Compile
        </button>
        <button
          type="button"
          disabled={model.compileStatus === "loading"}
          onClick={() => dispatch({ type: "format-requested" })}
        >
          Format
        </button>
      </div>

      <span class="active-path">
        {model.activePath}
        {model.dirty ? " *" : ""}
      </span>
      <button type="button" onClick={() => dispatch({ type: "share-requested" })}>
        Share
      </button>
    </div>
  );
}

function DiagnosticsPanel({ model, dispatch }: Props) {
  return (
    <aside class="diagnostics-panel" aria-label="Diagnostics">
      <header class="panel-header compact">
        <div>
          <p class="eyebrow">Compile</p>
          <h2>Diagnostics</h2>
        </div>
        <span class={`status-pill ${statusClass(model)}`}>{statusLabel(model)}</span>
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
