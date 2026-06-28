import { cloneExample, examples } from "../../data/examples";
import type { ZipWorkspace } from "../../workspace/zip";
import {
  compileCommand,
  confirmLoadExampleCommand,
  exportZipCommand,
  importZipCommand,
  shareCommand,
  stopPlaybackCommand,
} from "../commands";
import { currentWorkspace } from "../selectors";
import type { Model, UpdateResult } from "../types";
import { fail, workspaceDiagnostic } from "./state";

export function loadExampleRequested(model: Model, exampleId: string): UpdateResult {
  return model.dirty
    ? [model, [confirmLoadExampleCommand(exampleId)]]
    : loadExample(model, exampleId);
}

export function loadExample(model: Model, exampleId: string): UpdateResult {
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
    isPlaybackLoading: false,
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

export function applyImportedWorkspace(model: Model, workspace: ZipWorkspace): UpdateResult {
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
    isPlaybackLoading: false,
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

export function loadExampleConfirmed(model: Model, exampleId: string, confirmed: boolean): UpdateResult {
  return confirmed ? loadExample(model, exampleId) : [model, []];
}

export function shareRequested(model: Model): UpdateResult {
  return [model, [shareCommand(currentWorkspace(model))]];
}

export function shareFinished(model: Model, message: string): UpdateResult {
  return [{ ...model, diagnostics: [workspaceDiagnostic(message)] }, []];
}

export function exportZipRequested(model: Model): UpdateResult {
  return [model, [exportZipCommand(model.files)]];
}

export function importZipSelected(model: Model, file: File): UpdateResult {
  return [model, [importZipCommand(file, model.dirty)]];
}

export function workspaceError(model: Model, message: string): UpdateResult {
  return [fail(model, message), []];
}

export function initialExampleFromUrl(search: string, restoredWorkspace: ZipWorkspace | null) {
  if (restoredWorkspace) {
    return examples[0];
  }

  const requestedExampleId = new URLSearchParams(search).get("example");
  return examples.find((example) => example.id === requestedExampleId) ?? examples[0];
}
