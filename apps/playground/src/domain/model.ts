import { cloneExample } from "../data/examples";
import type { ZipWorkspace } from "../workspace/zip";
import { restoreWorkspaceFromHash } from "../share/url";
import { initWasmCommand } from "./commands";
import {
  compileFinished,
  compileRequested,
  compilerLoaded,
  compilerLoadFailed,
  formatFinished,
  formatRequested,
} from "./reducers/compiler";
import { fileSelected, pendingCursorApplied, selectDiagnostic, sourceChanged } from "./reducers/editor";
import {
  deleteConfirmed,
  deleteRequested,
  fragmentAppendConfirmed,
  fragmentNameSubmitted,
  fragmentRequested,
  newFileRequested,
  newFileSubmitted,
  renameRequested,
  renameSubmitted,
  setEntryRequested,
} from "./reducers/file";
import {
  playbackEnded,
  playbackFailed,
  playbackStarted,
  playbackTick,
  playRequested,
  stopRequested,
} from "./reducers/playback";
import { workspaceDiagnostic } from "./reducers/state";
import {
  applyImportedWorkspace,
  exportZipRequested,
  importZipSelected,
  initialExampleFromUrl,
  loadExampleConfirmed,
  loadExampleRequested,
  shareFinished,
  shareRequested,
  workspaceError,
} from "./reducers/workspace";
import type { Message, Model, UpdateResult } from "./types";

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
export { workspaceDiagnostic } from "./reducers/state";

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
    case "compiler-loaded":
      return compilerLoaded(model);
    case "compiler-load-failed":
      return compilerLoadFailed(model, message.message);
    case "compile-requested":
      return compileRequested(model, message.reason);
    case "compile-finished":
      return compileFinished(model, message.output, message.reason);
    case "format-requested":
      return formatRequested(model);
    case "format-finished":
      return formatFinished(model, message.output);
    case "source-changed":
      return sourceChanged(model, message.path, message.content);
    case "file-selected":
      return fileSelected(model, message.path);
    case "diagnostic-selected":
      return selectDiagnostic(model, message.index);
    case "pending-cursor-applied":
      return pendingCursorApplied(model);
    case "new-file-requested":
      return newFileRequested(model);
    case "new-file-submitted":
      return newFileSubmitted(model, message.path);
    case "fragment-requested":
      return fragmentRequested(model);
    case "fragment-name-submitted":
      return fragmentNameSubmitted(model, message.name);
    case "fragment-append-confirmed":
      return fragmentAppendConfirmed(model, message.name, message.appendCall);
    case "rename-requested":
      return renameRequested(model);
    case "rename-submitted":
      return renameSubmitted(model, message.path);
    case "delete-requested":
      return deleteRequested(model);
    case "delete-confirmed":
      return deleteConfirmed(model, message.confirmed);
    case "set-entry-requested":
      return setEntryRequested(model);
    case "load-example-requested":
      return loadExampleRequested(model, message.exampleId);
    case "load-example-confirmed":
      return loadExampleConfirmed(model, message.exampleId, message.confirmed);
    case "share-requested":
      return shareRequested(model);
    case "share-finished":
      return shareFinished(model, message.message);
    case "export-zip-requested":
      return exportZipRequested(model);
    case "import-zip-selected":
      return importZipSelected(model, message.file);
    case "import-zip-finished":
      return applyImportedWorkspace(model, message.workspace);
    case "workspace-error":
      return workspaceError(model, message.message);
    case "play-requested":
      return playRequested(model);
    case "playback-started":
      return playbackStarted(model);
    case "playback-ended":
      return playbackEnded(model);
    case "playback-failed":
      return playbackFailed(model, message.message);
    case "stop-requested":
      return stopRequested(model);
    case "playback-tick":
      return playbackTick(model, message.position);
  }
}
