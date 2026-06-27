import { examples } from "../data/examples";
import type { PlaygroundFile } from "../workspace/types";
import type { ZipWorkspace } from "../workspace/zip";
import type { Diagnostic, Model } from "./types";

export function currentFile(model: Model): PlaygroundFile {
  const file = findFile(model, model.activePath);
  if (!file) {
    throw new Error(`Missing active file: ${model.activePath}`);
  }
  return file;
}

export function findFile(model: Model, path: string): PlaygroundFile | undefined {
  return model.files.find((file) => file.path === path);
}

export function currentExample(model: Model) {
  return examples.find((example) => example.id === model.currentExampleId);
}

export function currentWorkspace(model: Model): ZipWorkspace {
  return {
    entryPath: model.entryPath,
    activePath: model.activePath,
    files: model.files,
  };
}

export function diagnosticLocation(diagnostic: Diagnostic): string {
  const path = diagnostic.path ?? "workspace";
  const line = diagnostic.line ? `:${diagnostic.line}` : "";
  const column = diagnostic.column ? `:${diagnostic.column}` : "";
  return `${path}${line}${column}`;
}
