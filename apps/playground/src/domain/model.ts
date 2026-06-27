import { cloneExample, examples } from "../data/examples";
import { createFragmentChange } from "../workspace/fragments";
import { validateWorkspacePath, type ZipWorkspace } from "../workspace/zip";
import { restoreWorkspaceFromHash } from "../share/url";
import {
  compileCommand,
  confirmDeleteCommand,
  confirmFragmentAppendCommand,
  confirmLoadExampleCommand,
  exportZipCommand,
  formatCommand,
  importZipCommand,
  initWasmCommand,
  playCommand,
  promptFragmentNameCommand,
  promptNewFileCommand,
  promptRenameCommand,
  shareCommand,
  stopPlaybackCommand,
} from "./commands";
import { currentFile, currentWorkspace, findFile } from "./selectors";
import { playbackOptions } from "./playback";
import type { CompileOutput, CompileReason, Diagnostic, FormatOutput, Message, Model, UpdateResult } from "./types";

export type {
  Command,
  CompileOutput,
  CompileReason,
  CompileStatus,
  Diagnostic,
  Dispatch,
  Effects,
  FormatOutput,
  Message,
  Model,
  PlaybackOptions,
  PlaybackPosition,
  PlaygroundMetadata,
  UpdateResult,
} from "./types";
export { currentExample, currentFile, currentWorkspace, diagnosticLocation, findFile } from "./selectors";
export { playbackOptions } from "./playback";

export function initModel(location: Pick<Location, "hash" | "search">): UpdateResult {
  let restoreError: string | null = null;
  let restoredWorkspace: ZipWorkspace | null = null;
  try {
    restoredWorkspace = restoreWorkspaceFromHash(location.hash);
  } catch (error) {
    restoreError = `Cannot restore share URL: ${
      error instanceof Error ? error.message : String(error)
    }`;
  }

  const initialExample = cloneExample(initialExampleFromUrl(location.search, restoredWorkspace));
  const model: Model = {
    files: restoredWorkspace?.files ?? initialExample.files,
    entryPath: restoredWorkspace?.entryPath ?? initialExample.entryPath,
    activePath: restoredWorkspace?.activePath ?? initialExample.activePath,
    diagnostics: restoreError ? [workspaceDiagnostic(restoreError)] : [],
    compileStatus: "loading",
    eventCount: 0,
    compiledEvents: [],
    metadata: undefined,
    isPlaying: false,
    playbackPosition: undefined,
    currentExampleId: restoredWorkspace ? "custom" : initialExample.id,
    dirty: false,
  };

  return [model, [initWasmCommand]];
}

export function update(model: Model, message: Message): UpdateResult {
  switch (message.type) {
    case "compiler-loaded": {
      if (model.compileStatus === "err" && model.diagnostics.length > 0) {
        return [model, []];
      }
      const next = { ...model, compileStatus: "ready" as const };
      return [next, [compileCommand("boot", currentWorkspace(next))]];
    }
    case "compiler-load-failed":
      return [
        fail(model, `Failed to load Loom compiler: ${message.message}`),
        [],
      ];
    case "compile-requested":
      return [
        { ...model, isPlaying: false, playbackPosition: undefined },
        [
          stopPlaybackCommand,
          compileCommand(message.reason, currentWorkspace(model)),
        ],
      ];
    case "compile-finished":
      return compileFinished(model, message.output, message.reason);
    case "format-requested":
      return [model, [formatCommand(currentFile(model).content)]];
    case "format-finished":
      return formatFinished(model, message.output);
    case "source-changed":
      return [markDirty(updateFileContent(model, message.path, message.content)), [stopPlaybackCommand]];
    case "file-selected":
      return [{ ...model, activePath: message.path }, []];
    case "diagnostic-selected":
      return selectDiagnostic(model, message.index);
    case "pending-cursor-applied":
      return [{ ...model, pendingCursor: undefined }, []];
    case "new-file-requested":
      return [model, [promptNewFileCommand]];
    case "new-file-submitted":
      return newFileSubmitted(model, message.path);
    case "fragment-requested":
      if (model.activePath !== model.entryPath) {
        return [fail(model, "Open the manifest entry file before creating a fragment."), []];
      }
      return [model, [promptFragmentNameCommand]];
    case "fragment-name-submitted":
      return message.name
        ? [model, [confirmFragmentAppendCommand(message.name, model.entryPath)]]
        : [model, []];
    case "fragment-append-confirmed":
      return fragmentAppendConfirmed(model, message.name, message.appendCall);
    case "rename-requested":
      return [model, [promptRenameCommand(model.activePath)]];
    case "rename-submitted":
      return renameSubmitted(model, message.path);
    case "delete-requested":
      return model.files.length <= 1
        ? [model, []]
        : [model, [confirmDeleteCommand(model.activePath)]];
    case "delete-confirmed":
      return deleteConfirmed(model, message.confirmed);
    case "set-entry-requested":
      return [markDirty({ ...model, entryPath: model.activePath, currentExampleId: "custom" }), [stopPlaybackCommand]];
    case "load-example-requested":
      return model.dirty
        ? [model, [confirmLoadExampleCommand(message.exampleId)]]
        : loadExample(model, message.exampleId);
    case "load-example-confirmed":
      return message.confirmed ? loadExample(model, message.exampleId) : [model, []];
    case "share-requested":
      return [model, [shareCommand(currentWorkspace(model))]];
    case "share-finished":
      return [{ ...model, diagnostics: [workspaceDiagnostic(message.message)] }, []];
    case "export-zip-requested":
      return [model, [exportZipCommand(model.files)]];
    case "import-zip-selected":
      return [model, [importZipCommand(message.file, model.dirty)]];
    case "import-zip-finished":
      return applyImportedWorkspace(model, message.workspace);
    case "workspace-error":
      return [fail(model, message.message), []];
    case "play-requested":
      return playRequested(model);
    case "playback-started":
      return [{ ...model, isPlaying: true }, []];
    case "playback-ended":
      return [{ ...model, isPlaying: false, playbackPosition: undefined }, []];
    case "playback-failed":
      return [fail({ ...model, isPlaying: false, playbackPosition: undefined }, message.message), []];
    case "stop-requested":
      return [{ ...model, isPlaying: false, playbackPosition: undefined }, [stopPlaybackCommand]];
    case "playback-tick":
      return [{ ...model, playbackPosition: message.position }, []];
  }
}

function compileFinished(model: Model, output: CompileOutput, reason: CompileReason): UpdateResult {
  if (output.status === "err") {
    return [
      {
        ...model,
        compileStatus: "err",
        eventCount: 0,
        compiledEvents: [],
        metadata: undefined,
        diagnostics: output.diagnostics,
      },
      [],
    ];
  }

  const next: Model = {
    ...model,
    compileStatus: "ok",
    eventCount: output.events.length,
    compiledEvents: output.events,
    metadata: output.metadata,
    diagnostics: [],
  };

  return reason === "play" ? playRequested(next) : [next, []];
}

function formatFinished(model: Model, output: FormatOutput): UpdateResult {
  if (output.status === "err") {
    return [
      {
        ...model,
        compileStatus: "err",
        eventCount: 0,
        compiledEvents: [],
        metadata: undefined,
        diagnostics: output.diagnostics.map((diagnostic) => ({
          ...diagnostic,
          path: diagnostic.path ?? model.activePath,
        })),
      },
      [],
    ];
  }

  const next = updateFileContent(model, model.activePath, output.source);
  return [
    { ...next, dirty: true },
    [compileCommand("manual", currentWorkspace(next))],
  ];
}

function newFileSubmitted(model: Model, path: string | null): UpdateResult {
  const nextPath = path?.trim();
  if (!nextPath) {
    return [model, []];
  }
  const validation = validatePath(nextPath);
  if (validation) {
    return [fail(model, validation), []];
  }
  if (findFile(model, nextPath)) {
    return [fail(model, `File already exists: ${nextPath}`), []];
  }

  return [
    markDirty({
      ...model,
      files: [...model.files, { path: nextPath, content: "# 1\nC4 | ^ |\n" }],
      activePath: nextPath,
      currentExampleId: "custom",
    }),
    [stopPlaybackCommand],
  ];
}

function fragmentAppendConfirmed(model: Model, name: string, appendCall: boolean): UpdateResult {
  try {
    const manifest = currentFile(model);
    const change = createFragmentChange({
      manifest,
      existingPaths: new Set(model.files.map((file) => file.path)),
      name,
      appendCall,
    });
    const files = model.files
      .map((file) => (file.path === manifest.path ? { ...file, content: change.manifestContent } : file))
      .concat(change.fragment)
      .sort((left, right) => left.path.localeCompare(right.path));
    return [
      markDirty({ ...model, files, activePath: change.fragment.path, currentExampleId: "custom" }),
      [stopPlaybackCommand],
    ];
  } catch (error) {
    return [fail(model, error instanceof Error ? error.message : String(error)), []];
  }
}

function renameSubmitted(model: Model, path: string | null): UpdateResult {
  const nextPath = path?.trim();
  if (!nextPath || nextPath === model.activePath) {
    return [model, []];
  }
  const validation = validatePath(nextPath);
  if (validation) {
    return [fail(model, validation), []];
  }
  if (findFile(model, nextPath)) {
    return [fail(model, `File already exists: ${nextPath}`), []];
  }

  return [
    markDirty({
      ...model,
      files: model.files.map((file) => file.path === model.activePath ? { ...file, path: nextPath } : file),
      activePath: nextPath,
      entryPath: model.entryPath === model.activePath ? nextPath : model.entryPath,
      currentExampleId: "custom",
    }),
    [stopPlaybackCommand],
  ];
}

function deleteConfirmed(model: Model, confirmed: boolean): UpdateResult {
  if (!confirmed || model.files.length <= 1) {
    return [model, []];
  }

  const files = model.files.filter((file) => file.path !== model.activePath);
  const entryPath = model.entryPath === model.activePath ? files[0]?.path ?? "" : model.entryPath;
  const activePath = files.find((file) => file.path === entryPath)?.path ?? files[0].path;

  return [
    markDirty({ ...model, files, entryPath, activePath, currentExampleId: "custom" }),
    [stopPlaybackCommand],
  ];
}

function loadExample(model: Model, exampleId: string): UpdateResult {
  const example = examples.find((candidate) => candidate.id === exampleId);
  if (!example) {
    return [model, []];
  }

  const nextExample = cloneExample(example);
  const next: Model = {
    ...model,
    files: nextExample.files,
    entryPath: nextExample.entryPath,
    activePath: nextExample.activePath,
    currentExampleId: nextExample.id,
    dirty: false,
    diagnostics: [],
    compiledEvents: [],
    eventCount: 0,
    metadata: undefined,
    compileStatus: "ready",
    isPlaying: false,
    playbackPosition: undefined,
  };
  return [
    next,
    [
      stopPlaybackCommand,
      compileCommand("load-workspace", currentWorkspace(next)),
    ],
  ];
}

function applyImportedWorkspace(model: Model, workspace: ZipWorkspace): UpdateResult {
  const next: Model = {
    ...model,
    files: workspace.files,
    entryPath: workspace.entryPath,
    activePath: workspace.activePath,
    currentExampleId: "custom",
    dirty: false,
    diagnostics: [],
    compiledEvents: [],
    eventCount: 0,
    metadata: undefined,
    compileStatus: "ready",
    isPlaying: false,
    playbackPosition: undefined,
  };
  return [
    next,
    [
      stopPlaybackCommand,
      compileCommand("load-workspace", currentWorkspace(next)),
    ],
  ];
}

function playRequested(model: Model): UpdateResult {
  if (model.compileStatus !== "ok") {
    return [
      model,
      [
        stopPlaybackCommand,
        compileCommand("play", currentWorkspace(model)),
      ],
    ];
  }

  const notes = model.compiledEvents.flatMap((event) => ("Note" in event ? [event.Note] : []));
  if (notes.length === 0) {
    return [fail(model, "Nothing to play: no note events were compiled."), []];
  }

  return [model, [playCommand(model.compiledEvents, playbackOptions(model))]];
}

function markDirty(model: Model): Model {
  return {
    ...model,
    compileStatus: "dirty",
    eventCount: 0,
    diagnostics: [],
    compiledEvents: [],
    metadata: undefined,
    dirty: true,
    isPlaying: false,
    playbackPosition: undefined,
  };
}

function fail(model: Model, message: string): Model {
  return {
    ...model,
    compileStatus: "err",
    eventCount: 0,
    compiledEvents: [],
    metadata: undefined,
    diagnostics: [workspaceDiagnostic(message)],
  };
}

function selectDiagnostic(model: Model, index: number): UpdateResult {
  const diagnostic = model.diagnostics[index];
  if (!diagnostic?.path || !findFile(model, diagnostic.path)) {
    return [model, []];
  }
  return [
    {
      ...model,
      activePath: diagnostic.path,
      pendingCursor: {
        path: diagnostic.path,
        line: diagnostic.line ?? 1,
        column: diagnostic.column ?? 1,
      },
    },
    [],
  ];
}

function updateFileContent(model: Model, path: string, content: string): Model {
  return {
    ...model,
    files: model.files.map((file) => file.path === path ? { ...file, content } : file),
  };
}

export function workspaceDiagnostic(message: string): Diagnostic {
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

function validatePath(path: string): string | null {
  try {
    validateWorkspacePath(path);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

function initialExampleFromUrl(search: string, restoredWorkspace: ZipWorkspace | null) {
  if (restoredWorkspace) {
    return examples[0];
  }

  const requestedExampleId = new URLSearchParams(search).get("example");
  return examples.find((example) => example.id === requestedExampleId) ?? examples[0];
}
