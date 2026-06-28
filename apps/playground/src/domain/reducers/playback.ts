import { compileCommand, playCommand, stopPlaybackCommand } from "../commands";
import { playbackOptions } from "../playback";
import { currentWorkspace } from "../selectors";
import type { Model, PlaybackPosition, UpdateResult } from "../types";
import { fail } from "./state";

export function playRequested(model: Model): UpdateResult {
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

  return [{ ...model, isPlaybackLoading: true }, [playCommand(model.compiledEvents, playbackOptions(model))]];
}

export function playbackLoading(model: Model): UpdateResult {
  return [{ ...model, isPlaybackLoading: true, playbackPosition: undefined }, []];
}

export function playbackStarted(model: Model): UpdateResult {
  return [{ ...model, isPlaying: true, isPlaybackLoading: false }, []];
}

export function playbackEnded(model: Model): UpdateResult {
  return [{ ...model, isPlaying: false, isPlaybackLoading: false, playbackPosition: undefined }, []];
}

export function playbackFailed(model: Model, message: string): UpdateResult {
  return [fail({ ...model, isPlaying: false, isPlaybackLoading: false, playbackPosition: undefined }, message), []];
}

export function stopRequested(model: Model): UpdateResult {
  return [{ ...model, isPlaying: false, isPlaybackLoading: false, playbackPosition: undefined }, [stopPlaybackCommand]];
}

export function playbackTick(model: Model, position: PlaybackPosition | undefined): UpdateResult {
  return [{ ...model, playbackPosition: position }, []];
}
