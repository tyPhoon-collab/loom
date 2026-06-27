import { compileCommand, formatCommand, stopPlaybackCommand } from "../commands";
import { currentFile, currentWorkspace } from "../selectors";
import type { CompileOutput, CompileReason, FormatOutput, Model, UpdateResult } from "../types";
import { playRequested } from "./playback";
import { fail, updateFileContent } from "./state";

export function compilerLoaded(model: Model): UpdateResult {
  if (model.compileStatus === "err" && model.diagnostics.length > 0) {
    return [model, []];
  }
  const next = { ...model, compileStatus: "ready" as const };
  return [next, [compileCommand("boot", currentWorkspace(next))]];
}

export function compilerLoadFailed(model: Model, message: string): UpdateResult {
  return [
    fail(model, `Failed to load Loom compiler: ${message}`),
    [],
  ];
}

export function compileRequested(model: Model, reason: CompileReason): UpdateResult {
  return [
    { ...model, isPlaying: false, playbackPosition: undefined },
    [
      stopPlaybackCommand,
      compileCommand(reason, currentWorkspace(model)),
    ],
  ];
}

export function compileFinished(model: Model, output: CompileOutput, reason: CompileReason): UpdateResult {
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

export function formatFinished(model: Model, output: FormatOutput): UpdateResult {
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

export function formatRequested(model: Model): UpdateResult {
  return [model, [formatCommand(currentFile(model).content)]];
}
