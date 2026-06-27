import { createFragmentChange } from "../../workspace/fragments";
import {
  confirmDeleteCommand,
  confirmFragmentAppendCommand,
  promptFragmentNameCommand,
  promptNewFileCommand,
  promptRenameCommand,
  stopPlaybackCommand,
} from "../commands";
import { currentFile, findFile } from "../selectors";
import type { Model, UpdateResult } from "../types";
import { fail, markDirty, validatePath } from "./state";

export function newFileRequested(model: Model): UpdateResult {
  return [model, [promptNewFileCommand]];
}

export function newFileSubmitted(model: Model, path: string | null): UpdateResult {
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

export function fragmentAppendConfirmed(model: Model, name: string, appendCall: boolean): UpdateResult {
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

export function fragmentRequested(model: Model): UpdateResult {
  if (model.activePath !== model.entryPath) {
    return [fail(model, "Open the manifest entry file before creating a fragment."), []];
  }
  return [model, [promptFragmentNameCommand]];
}

export function fragmentNameSubmitted(model: Model, name: string | null): UpdateResult {
  return name
    ? [model, [confirmFragmentAppendCommand(name, model.entryPath)]]
    : [model, []];
}

export function renameRequested(model: Model): UpdateResult {
  return [model, [promptRenameCommand(model.activePath)]];
}

export function renameSubmitted(model: Model, path: string | null): UpdateResult {
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

export function deleteConfirmed(model: Model, confirmed: boolean): UpdateResult {
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

export function deleteRequested(model: Model): UpdateResult {
  return model.files.length <= 1
    ? [model, []]
    : [model, [confirmDeleteCommand(model.activePath)]];
}

export function setEntryRequested(model: Model): UpdateResult {
  return [markDirty({ ...model, entryPath: model.activePath, currentExampleId: "custom" }), [stopPlaybackCommand]];
}
