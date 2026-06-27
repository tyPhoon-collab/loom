import type { Model, PlaybackOptions } from "./types";

export function playbackOptions(model: Model): PlaybackOptions {
  const metadata = model.metadata ?? {
    bpm: 120,
    unit: "bar",
    signature: "4/4",
    loop: false,
    loop_range: null,
  };
  const beatsPerUnit = metadata.unit === "beat" ? 1 : beatsPerBar(metadata.signature);

  return {
    bpm: metadata.bpm,
    loop: metadata.loop,
    loopRange: parseLoopRange(metadata.loop_range ?? undefined, beatsPerUnit),
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
