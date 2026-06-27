import "@knadh/oat/oat.min.css";
import { EditorState, RangeSetBuilder, type Extension } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import {
  isNotePreviewPlaying,
  notePreviewPosition,
  playNotePreview,
  stopNotePreview,
} from "./audio/web-audio";
import type { MidiEvent } from "./compiler/types";
import { basicSetup } from "codemirror";
import { cloneExample, examples } from "./examples";
import initWasm, {
  compileWorkspace,
  formatFile,
} from "./generated/loom_wasm/loom.js";
import {
  createShareUrl,
  restoreWorkspaceFromHash,
} from "./share/url";
import { createFragmentChange } from "./workspace/fragments";
import type { PlaygroundFile } from "./workspace/types";
import {
  exportWorkspaceZip,
  importWorkspaceZip,
  validateWorkspacePath,
  type ZipWorkspace,
} from "./workspace/zip";
import "./styles.css";

type Diagnostic = {
  path?: string | null;
  line?: number | null;
  column?: number | null;
  byte_offset?: number | null;
  length: number;
  severity: "error" | "warning";
  message: string;
  help?: string | null;
};

type CompileOutput =
  | {
      status: "ok";
      events: MidiEvent[];
      metadata: PlaygroundMetadata;
    }
  | {
      status: "err";
      diagnostics: Diagnostic[];
    };

type FormatOutput =
  | {
      status: "ok";
      source: string;
    }
  | {
      status: "err";
      diagnostics: Diagnostic[];
    };

type CompileStatus = "loading" | "ready" | "dirty" | "ok" | "err";

type PlaygroundMetadata = {
  bpm: number;
  signature: string;
  unit: string;
  loop: boolean;
  loop_range?: string | null;
};

type AppState = {
  files: PlaygroundFile[];
  entryPath: string;
  activePath: string;
  diagnostics: Diagnostic[];
  compileStatus: CompileStatus;
  eventCount: number;
  compiledEvents: MidiEvent[];
  metadata?: PlaygroundMetadata;
  isPlaying: boolean;
  playbackPosition?: {
    beat: number;
    seconds: number;
    loop: boolean;
  };
  currentExampleId: string;
  dirty: boolean;
  pendingCursor?: {
    path: string;
    line: number;
    column: number;
  };
};

let initialRestoreError: string | null = null;
const restoredWorkspace = restoreInitialWorkspace();
const initialExample = cloneExample(initialExampleFromUrl());

const state: AppState = {
  files: restoredWorkspace?.files ?? initialExample.files,
  entryPath: restoredWorkspace?.entryPath ?? initialExample.entryPath,
  activePath: restoredWorkspace?.activePath ?? initialExample.activePath,
  diagnostics: initialRestoreError ? [workspaceDiagnostic(initialRestoreError)] : [],
  compileStatus: "loading",
  eventCount: 0,
  compiledEvents: [],
  metadata: undefined,
  isPlaying: false,
  playbackPosition: undefined,
  currentExampleId: restoredWorkspace ? "custom" : initialExample.id,
  dirty: false,
};

const appRoot = document.querySelector<HTMLDivElement>("#app");

if (!appRoot) {
  throw new Error("Missing #app root");
}

const app = appRoot;
let editorView: EditorView | null = null;
let playbackStatusTimer: number | null = null;

void boot();

async function boot(): Promise<void> {
  render();
  try {
    await initWasm();
    if (initialRestoreError) {
      state.compileStatus = "err";
      render();
      return;
    }
    state.compileStatus = "ready";
    runCompile();
  } catch (error) {
    state.compileStatus = "err";
    state.diagnostics = [
      workspaceDiagnostic(`Failed to load Loom compiler: ${String(error)}`),
    ];
    render();
  }
}

function render(): void {
  const activeFile = currentFile();
  editorView?.destroy();
  editorView = null;
  app.innerHTML = `
    <main class="playground-shell">
      <aside class="workspace-panel" aria-label="Workspace files">
        <header class="panel-header">
          <div>
            <p class="eyebrow">Workspace</p>
            <h1>Loom Playground</h1>
          </div>
          <button type="button" class="small" data-action="new-file">New</button>
        </header>
        <div class="example-picker">
          <label for="example-select">Example</label>
          <select id="example-select" data-action="load-example">
            ${
              currentExample()
                ? ""
                : '<option value="custom" selected>Custom workspace</option>'
            }
            ${examples
              .map(
                (example) => `
                  <option value="${escapeAttribute(example.id)}" ${
                    example.id === state.currentExampleId ? "selected" : ""
                  }>${escapeHtml(example.name)}</option>
                `,
              )
              .join("")}
          </select>
          <p>${escapeHtml(currentExample()?.description ?? "Custom workspace")}</p>
        </div>
        <nav class="file-list" aria-label="Files">
          ${state.files
            .map(
              (file) => `
                <button type="button" class="file-item ${
                  file.path === state.activePath ? "active" : ""
                }" data-file-path="${escapeAttribute(file.path)}">
                  <span>${escapeHtml(file.path)}</span>
                  ${file.path === state.entryPath ? '<em title="Entry file">entry</em>' : ""}
                </button>
              `,
            )
            .join("")}
        </nav>
      </aside>

      <section class="editor-panel" aria-label="Loom source editor">
        <div class="toolbar" role="toolbar" aria-label="Playground actions">
          <button type="button" data-action="play" ${playDisabled() ? "disabled" : ""}>Play</button>
          <button type="button" data-action="stop" ${state.isPlaying ? "" : "disabled"}>Stop</button>
          <button type="button" data-action="compile" ${state.compileStatus === "loading" ? "disabled" : ""}>Compile</button>
          <button type="button" data-action="format" ${state.compileStatus === "loading" ? "disabled" : ""}>Format</button>
          <button type="button" data-action="new-fragment" ${state.activePath === state.entryPath ? "" : "disabled"}>New Fragment</button>
          <button type="button" data-action="set-entry" ${state.activePath === state.entryPath ? "disabled" : ""}>Set Entry</button>
          <button type="button" data-action="rename-file">Rename</button>
          <button type="button" data-action="delete-file" ${state.files.length <= 1 ? "disabled" : ""}>Delete</button>
          <button type="button" data-action="share">Share</button>
          <button type="button" data-action="export-zip">Export ZIP</button>
          <button type="button" data-action="import-zip">Import ZIP</button>
          <span class="active-path">${escapeHtml(state.activePath)}${state.dirty ? " *" : ""}</span>
        </div>
        <input type="file" data-zip-input accept=".zip,application/zip" hidden />
        <div class="editor-host" data-editor-host aria-label="${escapeAttribute(
          state.activePath,
        )} source"></div>
      </section>

      <aside class="diagnostics-panel" aria-label="Diagnostics">
        <header class="panel-header compact">
          <div>
            <p class="eyebrow">Compile</p>
            <h2>Diagnostics</h2>
          </div>
          <span class="status-pill ${statusClass()}">${statusLabel()}</span>
        </header>
        <div data-diagnostics-body>
          ${renderDiagnostics()}
        </div>
      </aside>
    </main>
  `;

  bindEvents();
  mountEditor(activeFile);
  applyPendingCursor();
}

function renderDiagnostics(): string {
  if (state.diagnostics.length === 0) {
    const message =
      state.compileStatus === "ok"
        ? `${state.eventCount} MIDI events compiled.`
        : "No diagnostics.";
    return `<div class="empty-state">${escapeHtml(message)}</div>`;
  }

  return `
    <ol class="diagnostics-list">
      ${state.diagnostics
        .map(
          (diagnostic, index) => `
            <li>
              <button type="button" data-diagnostic-index="${index}">
                <strong>${escapeHtml(diagnosticLocation(diagnostic))}</strong>
                <span>${escapeHtml(diagnostic.message)}</span>
                ${diagnostic.help ? `<small>${escapeHtml(diagnostic.help)}</small>` : ""}
              </button>
            </li>
          `,
        )
        .join("")}
    </ol>
  `;
}

function bindEvents(): void {
  app.querySelectorAll<HTMLButtonElement>("[data-file-path]").forEach((button) => {
    button.addEventListener("click", () => {
      const path = button.dataset.filePath;
      if (!path) {
        return;
      }
      state.activePath = path;
      render();
    });
  });

  app.querySelector<HTMLSelectElement>('[data-action="load-example"]')?.addEventListener("change", (event) => {
    const target = event.currentTarget as HTMLSelectElement;
    loadExample(target.value);
  });

  app.querySelector<HTMLButtonElement>('[data-action="play"]')?.addEventListener("click", () => {
    void playWorkspace();
  });

  app.querySelector<HTMLButtonElement>('[data-action="stop"]')?.addEventListener("click", () => {
    stopPlayback();
  });

  app.querySelector<HTMLButtonElement>('[data-action="compile"]')?.addEventListener("click", () => {
    runCompile();
  });

  app.querySelector<HTMLButtonElement>('[data-action="format"]')?.addEventListener("click", () => {
    runFormat();
  });

  app.querySelector<HTMLButtonElement>('[data-action="new-file"]')?.addEventListener("click", () => {
    createFile();
  });

  app.querySelector<HTMLButtonElement>('[data-action="new-fragment"]')?.addEventListener("click", () => {
    createFragment();
  });

  app.querySelector<HTMLButtonElement>('[data-action="rename-file"]')?.addEventListener("click", () => {
    renameActiveFile();
  });

  app.querySelector<HTMLButtonElement>('[data-action="delete-file"]')?.addEventListener("click", () => {
    deleteActiveFile();
  });

  app.querySelector<HTMLButtonElement>('[data-action="set-entry"]')?.addEventListener("click", () => {
    state.entryPath = state.activePath;
    state.currentExampleId = "custom";
    markDirty();
  });

  app.querySelector<HTMLButtonElement>('[data-action="share"]')?.addEventListener("click", () => {
    void shareWorkspace();
  });

  app.querySelector<HTMLButtonElement>('[data-action="export-zip"]')?.addEventListener("click", () => {
    exportZip();
  });

  app.querySelector<HTMLButtonElement>('[data-action="import-zip"]')?.addEventListener("click", () => {
    app.querySelector<HTMLInputElement>("[data-zip-input]")?.click();
  });

  app.querySelector<HTMLInputElement>("[data-zip-input]")?.addEventListener("change", (event) => {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) {
      return;
    }
    void importZip(file);
  });

  app.querySelectorAll<HTMLButtonElement>("[data-diagnostic-index]").forEach((button) => {
    button.addEventListener("click", () => {
      const index = Number(button.dataset.diagnosticIndex);
      const diagnostic = state.diagnostics[index];
      if (!diagnostic?.path || !findFile(diagnostic.path)) {
        return;
      }
      state.activePath = diagnostic.path;
      state.pendingCursor = {
        path: diagnostic.path,
        line: diagnostic.line ?? 1,
        column: diagnostic.column ?? 1,
      };
      render();
    });
  });
}

function runCompile(): boolean {
  stopPlayback({ renderAfter: false });
  const output = JSON.parse(
    compileWorkspace(
      JSON.stringify({
        entry_path: state.entryPath,
        active_path: state.activePath,
        files: Object.fromEntries(state.files.map((file) => [file.path, file.content])),
      }),
    ),
  ) as CompileOutput;

  if (output.status === "ok") {
    state.compileStatus = "ok";
    state.eventCount = output.events.length;
    state.compiledEvents = output.events;
    state.metadata = output.metadata;
    state.diagnostics = [];
  } else {
    state.compileStatus = "err";
    state.eventCount = 0;
    state.compiledEvents = [];
    state.metadata = undefined;
    state.diagnostics = output.diagnostics;
  }
  render();
  return output.status === "ok";
}

function runFormat(): void {
  const output = JSON.parse(formatFile(currentFile().content)) as FormatOutput;

  if (output.status === "ok") {
    currentFile().content = output.source;
    state.dirty = true;
    runCompile();
  } else {
    state.compileStatus = "err";
    state.eventCount = 0;
    state.compiledEvents = [];
    state.metadata = undefined;
    state.diagnostics = output.diagnostics.map((diagnostic) => ({
      ...diagnostic,
      path: diagnostic.path ?? state.activePath,
    }));
    render();
  }
}

function createFile(): void {
  const path = window.prompt("New .loom path", "sections/new.loom")?.trim();
  if (!path) {
    return;
  }
  if (!validatePath(path)) {
    return;
  }
  if (findFile(path)) {
    state.diagnostics = [workspaceDiagnostic(`File already exists: ${path}`)];
    state.compileStatus = "err";
    render();
    return;
  }
  state.files.push({ path, content: "# 1\nC4 | ^ |\n" });
  state.activePath = path;
  state.currentExampleId = "custom";
  markDirty();
}

function createFragment(): void {
  if (state.activePath !== state.entryPath) {
    state.diagnostics = [workspaceDiagnostic("Open the manifest entry file before creating a fragment.")];
    state.compileStatus = "err";
    render();
    return;
  }

  const name = window.prompt("New fragment name", "intro")?.trim();
  if (!name) {
    return;
  }

  const appendCall = window.confirm(`Add [[${name}]] to the end of ${state.entryPath}?`);

  try {
    const manifest = currentFile();
    const change = createFragmentChange({
      manifest,
      existingPaths: new Set(state.files.map((file) => file.path)),
      name,
      appendCall,
    });
    manifest.content = change.manifestContent;
    state.files.push(change.fragment);
    state.files.sort((left, right) => left.path.localeCompare(right.path));
    state.activePath = change.fragment.path;
    state.currentExampleId = "custom";
    markDirty();
  } catch (error) {
    state.diagnostics = [workspaceDiagnostic(error instanceof Error ? error.message : String(error))];
    state.compileStatus = "err";
    render();
  }
}

function renameActiveFile(): void {
  const activeFile = currentFile();
  const nextPath = window.prompt("Rename file", activeFile.path)?.trim();
  if (!nextPath || nextPath === activeFile.path) {
    return;
  }
  if (!validatePath(nextPath)) {
    return;
  }
  if (findFile(nextPath)) {
    state.diagnostics = [workspaceDiagnostic(`File already exists: ${nextPath}`)];
    state.compileStatus = "err";
    render();
    return;
  }
  const previousPath = activeFile.path;
  activeFile.path = nextPath;
  state.activePath = nextPath;
  state.currentExampleId = "custom";
  if (state.entryPath === previousPath) {
    state.entryPath = nextPath;
  }
  markDirty();
}

function deleteActiveFile(): void {
  if (state.files.length <= 1) {
    return;
  }
  const activePath = state.activePath;
  if (!window.confirm(`Delete ${activePath}?`)) {
    return;
  }
  state.files = state.files.filter((file) => file.path !== activePath);
  if (state.entryPath === activePath) {
    state.entryPath = state.files[0]?.path ?? "";
  }
  state.activePath = state.files.find((file) => file.path === state.entryPath)?.path ?? state.files[0].path;
  state.currentExampleId = "custom";
  markDirty();
}

function markDirty(): void {
  state.compileStatus = "dirty";
  state.eventCount = 0;
  state.diagnostics = [];
  state.compiledEvents = [];
  state.metadata = undefined;
  state.dirty = true;
  stopPlayback({ renderAfter: false });
  render();
}

function loadExample(exampleId: string): void {
  const example = examples.find((candidate) => candidate.id === exampleId);
  if (!example) {
    render();
    return;
  }

  if (state.dirty && !window.confirm("Discard current workspace changes and load this example?")) {
    render();
    return;
  }

  const nextExample = cloneExample(example);
  stopPlayback({ renderAfter: false });
  state.files = nextExample.files;
  state.entryPath = nextExample.entryPath;
  state.activePath = nextExample.activePath;
  state.currentExampleId = nextExample.id;
  state.dirty = false;
  state.diagnostics = [];
  state.compiledEvents = [];
  state.eventCount = 0;
  state.metadata = undefined;
  state.compileStatus = "ready";
  runCompile();
}

function refreshCompilePanel(): void {
  const status = app.querySelector<HTMLSpanElement>(".status-pill");
  if (status) {
    status.className = `status-pill ${statusClass()}`;
    status.textContent = statusLabel();
  }

  const diagnosticsBody = app.querySelector<HTMLElement>("[data-diagnostics-body]");
  if (diagnosticsBody) {
    diagnosticsBody.innerHTML = renderDiagnostics();
  }
}

function refreshToolbarState(): void {
  const activePath = app.querySelector<HTMLElement>(".active-path");
  if (activePath) {
    activePath.textContent = `${state.activePath}${state.dirty ? " *" : ""}`;
  }

  const playButton = app.querySelector<HTMLButtonElement>('[data-action="play"]');
  if (playButton) {
    playButton.disabled = playDisabled();
  }
}

function mountEditor(file: PlaygroundFile): void {
  const parent = app.querySelector<HTMLElement>("[data-editor-host]");
  if (!parent) {
    return;
  }

  editorView = new EditorView({
    parent,
    state: EditorState.create({
      doc: file.content,
      extensions: [
        basicSetup,
        EditorView.lineWrapping,
        diagnosticDecorations(),
        EditorView.updateListener.of(handleEditorUpdate),
      ],
    }),
  });
}

function handleEditorUpdate(update: ViewUpdate): void {
  if (!update.docChanged) {
    return;
  }

  currentFile().content = update.state.doc.toString();
  state.compileStatus = "dirty";
  state.eventCount = 0;
  state.diagnostics = [];
  state.compiledEvents = [];
  state.metadata = undefined;
  state.dirty = true;
  stopPlayback({ renderAfter: false });
  refreshToolbarState();
  refreshCompilePanel();
}

function diagnosticDecorations(): Extension {
  const activeDiagnostics = state.diagnostics.filter(
    (diagnostic) => diagnostic.path === state.activePath && diagnostic.line,
  );
  const ranges = activeDiagnostics
    .map((diagnostic) => diagnosticRange(currentFile().content, diagnostic))
    .filter((range): range is { lineStart: number; from: number; to: number } => range !== null)
    .sort((left, right) => left.lineStart - right.lineStart || left.from - right.from);
  const builder = new RangeSetBuilder<Decoration>();

  for (const range of ranges) {
    builder.add(range.lineStart, range.lineStart, Decoration.line({ class: "cm-diagnostic-line" }));
    builder.add(range.from, range.to, Decoration.mark({ class: "cm-diagnostic-mark" }));
  }

  return EditorView.decorations.of(builder.finish() as DecorationSet);
}

function diagnosticRange(
  source: string,
  diagnostic: Diagnostic,
): { lineStart: number; from: number; to: number } | null {
  if (!diagnostic.line) {
    return null;
  }

  const lineStart = offsetForLineColumn(source, diagnostic.line, 1);
  const from = offsetForLineColumn(source, diagnostic.line, diagnostic.column ?? 1);
  const lineEndIndex = source.indexOf("\n", lineStart);
  const lineEnd = lineEndIndex === -1 ? source.length : lineEndIndex;
  const to = Math.min(lineEnd, Math.max(from + 1, from + Math.max(1, diagnostic.length)));

  return {
    lineStart,
    from: Math.min(from, source.length),
    to: Math.min(to, source.length),
  };
}

function validatePath(path: string): boolean {
  try {
    validateWorkspacePath(path);
    return true;
  } catch (error) {
    state.diagnostics = [
      workspaceDiagnostic(error instanceof Error ? error.message : String(error)),
    ];
    state.compileStatus = "err";
    render();
    return false;
  }
}

function currentFile(): PlaygroundFile {
  const file = findFile(state.activePath);
  if (!file) {
    throw new Error(`Missing active file: ${state.activePath}`);
  }
  return file;
}

function findFile(path: string): PlaygroundFile | undefined {
  return state.files.find((file) => file.path === path);
}

function currentExample() {
  return examples.find((example) => example.id === state.currentExampleId);
}

function initialExampleFromUrl() {
  if (restoredWorkspace) {
    return examples[0];
  }

  const requestedExampleId = new URLSearchParams(window.location.search).get("example");
  return examples.find((example) => example.id === requestedExampleId) ?? examples[0];
}

function workspaceDiagnostic(message: string): Diagnostic {
  return {
    path: null,
    line: null,
    column: null,
    byte_offset: null,
    length: 0,
    severity: "error",
    message,
    help: null,
  };
}

function exportZip(): void {
  try {
    const data = exportWorkspaceZip(state.files);
    const buffer = new ArrayBuffer(data.byteLength);
    new Uint8Array(buffer).set(data);
    const url = URL.createObjectURL(new Blob([buffer], { type: "application/zip" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = "loom-playground.zip";
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    state.compileStatus = "err";
    state.diagnostics = [workspaceDiagnostic(error instanceof Error ? error.message : String(error))];
    render();
  }
}

async function importZip(file: File): Promise<void> {
  if (state.dirty && !window.confirm("Discard current workspace changes and import this ZIP?")) {
    return;
  }

  try {
    const workspace = importWorkspaceZip(new Uint8Array(await file.arrayBuffer()));
    const entryPath = chooseEntryPath(workspace);
    applyImportedWorkspace({ ...workspace, entryPath, activePath: entryPath });
  } catch (error) {
    state.compileStatus = "err";
    state.diagnostics = [workspaceDiagnostic(error instanceof Error ? error.message : String(error))];
    render();
  }
}

function chooseEntryPath(workspace: ZipWorkspace): string {
  if (workspace.files.some((file) => file.path === "song.loom")) {
    return "song.loom";
  }

  if (workspace.files.length === 1) {
    return workspace.entryPath;
  }

  const candidateText = workspace.files.map((file) => file.path).join("\n");
  const selectedPath = window.prompt(
    `Choose entry file from imported workspace:\n\n${candidateText}`,
    workspace.entryPath,
  );
  if (!selectedPath) {
    throw new Error("ZIP import cancelled: no entry file selected.");
  }
  if (!workspace.files.some((file) => file.path === selectedPath)) {
    throw new Error(`Entry file is not in imported ZIP: ${selectedPath}`);
  }
  return selectedPath;
}

function applyImportedWorkspace(workspace: ZipWorkspace): void {
  stopPlayback({ renderAfter: false });
  state.files = workspace.files;
  state.entryPath = workspace.entryPath;
  state.activePath = workspace.activePath;
  state.currentExampleId = "custom";
  state.dirty = false;
  state.diagnostics = [];
  state.compiledEvents = [];
  state.eventCount = 0;
  state.metadata = undefined;
  state.compileStatus = "ready";
  runCompile();
}

async function shareWorkspace(): Promise<void> {
  try {
    const shareUrl = createShareUrl(currentWorkspace(), window.location);
    if (!confirmSharePayloadSize(shareUrl.payloadLength)) {
      return;
    }

    await copyText(shareUrl.url);
    state.diagnostics = [
      workspaceDiagnostic(`Share URL copied. Payload: ${formatBytes(shareUrl.payloadLength)}.`),
    ];
    render();
  } catch (error) {
    state.compileStatus = "err";
    state.diagnostics = [workspaceDiagnostic(error instanceof Error ? error.message : String(error))];
    render();
  }
}

function confirmSharePayloadSize(payloadLength: number): boolean {
  if (payloadLength > 32 * 1024) {
    state.compileStatus = "err";
    state.diagnostics = [
      workspaceDiagnostic(
        `Share URL is too large (${formatBytes(payloadLength)}). Use Export ZIP instead.`,
      ),
    ];
    render();
    return false;
  }

  if (payloadLength > 16 * 1024) {
    return window.confirm(
      `This share URL is large (${formatBytes(payloadLength)}) and may break in chats or issue trackers. Copy it anyway?`,
    );
  }

  if (payloadLength > 8 * 1024) {
    return window.confirm(
      `This share URL is ${formatBytes(payloadLength)}. ZIP export is safer for larger workspaces. Copy it anyway?`,
    );
  }

  return true;
}

async function copyText(text: string): Promise<void> {
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(text);
    return;
  }
  window.prompt("Copy share URL", text);
}

function currentWorkspace(): ZipWorkspace {
  return {
    entryPath: state.entryPath,
    activePath: state.activePath,
    files: state.files,
  };
}

function restoreInitialWorkspace(): ZipWorkspace | null {
  try {
    return restoreWorkspaceFromHash(window.location.hash);
  } catch (error) {
    initialRestoreError = `Cannot restore share URL: ${
      error instanceof Error ? error.message : String(error)
    }`;
    return null;
  }
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} bytes`;
}

async function playWorkspace(): Promise<void> {
  if (state.compileStatus !== "ok" && !runCompile()) {
    return;
  }

  const notes = state.compiledEvents.flatMap((event) => ("Note" in event ? [event.Note] : []));
  if (notes.length === 0) {
    state.diagnostics = [workspaceDiagnostic("Nothing to play: no note events were compiled.")];
    state.compileStatus = "err";
    render();
    return;
  }

  try {
    await playNotePreview(notes, playbackOptions(), () => {
      state.isPlaying = false;
      state.playbackPosition = undefined;
      stopPlaybackStatusTimer();
      render();
    });
    state.isPlaying = true;
    startPlaybackStatusTimer();
    render();
  } catch (error) {
    state.isPlaying = false;
    state.playbackPosition = undefined;
    stopPlaybackStatusTimer();
    state.diagnostics = [workspaceDiagnostic(String(error))];
    state.compileStatus = "err";
    render();
  }
}

function stopPlayback(options: { renderAfter?: boolean } = {}): void {
  const { renderAfter = true } = options;
  stopNotePreview();
  state.isPlaying = false;
  state.playbackPosition = undefined;
  stopPlaybackStatusTimer();

  if (renderAfter) {
    render();
  }
}

function playbackOptions() {
  const metadata = state.metadata ?? {
    bpm: 120,
    unit: "bar",
    signature: "4/4",
    loop: false,
    loop_range: null,
  };
  const unit = metadata.unit;
  const signature = metadata.signature;
  const beatsPerUnit = unit === "beat" ? 1 : beatsPerBar(signature);
  const loopRange = parseLoopRange(metadata.loop_range ?? undefined, beatsPerUnit);

  return {
    bpm: metadata.bpm,
    loop: metadata.loop,
    loopRange,
  };
}

function parseLoopRange(
  value: string | undefined,
  beatsPerUnit: number,
): { startBeat: number; endBeat: number } | undefined {
  const match = value?.match(/^([0-9]+(?:\.[0-9]+)?)\.\.([0-9]+(?:\.[0-9]+)?)$/);
  if (!match) {
    return undefined;
  }

  return {
    startBeat: Number(match[1]) * beatsPerUnit,
    endBeat: Number(match[2]) * beatsPerUnit,
  };
}

function beatsPerBar(signature: string): number {
  const match = signature.match(/^(\d+)\/(\d+)$/);
  if (!match) {
    return 4;
  }
  return Number(match[1]) * (4 / Number(match[2]));
}

function startPlaybackStatusTimer(): void {
  stopPlaybackStatusTimer();
  const update = () => {
    state.playbackPosition = notePreviewPosition() ?? undefined;
    refreshCompilePanel();
  };
  update();
  playbackStatusTimer = window.setInterval(update, 250);
}

function stopPlaybackStatusTimer(): void {
  if (playbackStatusTimer === null) {
    return;
  }
  window.clearInterval(playbackStatusTimer);
  playbackStatusTimer = null;
}

function diagnosticLocation(diagnostic: Diagnostic): string {
  const path = diagnostic.path ?? "workspace";
  const line = diagnostic.line ? `:${diagnostic.line}` : "";
  const column = diagnostic.column ? `:${diagnostic.column}` : "";
  return `${path}${line}${column}`;
}

function applyPendingCursor(): void {
  if (!state.pendingCursor || state.pendingCursor.path !== state.activePath || !editorView) {
    return;
  }
  const offset = offsetForLineColumn(
    editorView.state.doc.toString(),
    state.pendingCursor.line,
    state.pendingCursor.column,
  );
  editorView.dispatch({
    selection: { anchor: offset },
    effects: EditorView.scrollIntoView(offset, { y: "center" }),
  });
  editorView.focus();
  state.pendingCursor = undefined;
}

function offsetForLineColumn(source: string, line: number, column: number): number {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < source.length) {
    const nextNewline = source.indexOf("\n", offset);
    if (nextNewline === -1) {
      return source.length;
    }
    offset = nextNewline + 1;
    currentLine += 1;
  }

  const lineEnd = source.indexOf("\n", offset);
  const end = lineEnd === -1 ? source.length : lineEnd;
  const chars = Array.from(source.slice(offset, end));
  return offset + chars.slice(0, Math.max(0, column - 1)).join("").length;
}

function statusLabel(): string {
  if (state.isPlaying || isNotePreviewPlaying()) {
    const position = state.playbackPosition;
    if (!position) {
      return "Playing";
    }
    return `Playing ${formatTime(position.seconds)} beat ${position.beat.toFixed(2)}${
      position.loop ? " loop" : ""
    }`;
  }

  switch (state.compileStatus) {
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

function formatTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${minutes}:${remainingSeconds}`;
}

function statusClass(): string {
  if (state.isPlaying || isNotePreviewPlaying()) {
    return "status-playing";
  }

  return `status-${state.compileStatus}`;
}

function playDisabled(): boolean {
  return state.compileStatus === "loading" || state.compileStatus === "err" || state.isPlaying;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttribute(value: string): string {
  return escapeHtml(value).replaceAll("'", "&#39;");
}
