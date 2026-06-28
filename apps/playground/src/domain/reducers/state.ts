import { validateWorkspacePath } from "../../workspace/zip";
import type { Diagnostic, Model } from "../types";

export function markDirty(model: Model): Model {
  return {
    ...model,
    compileStatus: "dirty",
    eventCount: 0,
    diagnostics: [],
    compiledEvents: [],
    metadata: undefined,
    dirty: true,
    isPlaying: false,
    isPlaybackLoading: false,
    playbackPosition: undefined,
  };
}

export function fail(model: Model, message: string): Model {
  return {
    ...model,
    compileStatus: "err",
    eventCount: 0,
    compiledEvents: [],
    metadata: undefined,
    diagnostics: [workspaceDiagnostic(message)],
  };
}

export function updateFileContent(model: Model, path: string, content: string): Model {
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

export function validatePath(path: string): string | null {
  try {
    validateWorkspacePath(path);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}
