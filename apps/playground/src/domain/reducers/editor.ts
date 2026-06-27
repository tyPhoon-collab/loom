import { stopPlaybackCommand } from "../commands";
import { findFile } from "../selectors";
import type { Model, UpdateResult } from "../types";
import { markDirty, updateFileContent } from "./state";

export function sourceChanged(model: Model, path: string, content: string): UpdateResult {
  return [markDirty(updateFileContent(model, path, content)), [stopPlaybackCommand]];
}

export function fileSelected(model: Model, path: string): UpdateResult {
  return [{ ...model, activePath: path }, []];
}

export function selectDiagnostic(model: Model, index: number): UpdateResult {
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

export function pendingCursorApplied(model: Model): UpdateResult {
  return [{ ...model, pendingCursor: undefined }, []];
}
