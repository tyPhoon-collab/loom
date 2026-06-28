import type { NoteEvent } from "../compiler/types";
import type { ChannelState } from "./channel-state";
import type { LoadedSample } from "./types";

export type PlaybackOutput = {
  master: DynamicsCompressorNode;
};

export function createPlaybackOutput(context: AudioContext): PlaybackOutput {
  const master = context.createDynamicsCompressor();
  master.threshold.setValueAtTime(-12, context.currentTime);
  master.knee.setValueAtTime(24, context.currentTime);
  master.ratio.setValueAtTime(8, context.currentTime);
  master.attack.setValueAtTime(0.003, context.currentTime);
  master.release.setValueAtTime(0.18, context.currentTime);
  master.connect(context.destination);
  return { master };
}

export function scheduleSample(
  context: AudioContext,
  output: PlaybackOutput,
  sample: LoadedSample,
  note: NoteEvent,
  state: ChannelState,
  startTime: number,
  secondsPerBeat: number,
): AudioBufferSourceNode {
  const source = context.createBufferSource();
  const gain = context.createGain();
  const pan = context.createStereoPanner();
  const duration = Math.max(0.04, note.duration * secondsPerBeat);
  const stopTime = startTime + duration;
  const velocity = Math.max(0, Math.min(127, note.velocity)) / 127;
  const sampleGain = sample.gain ?? 1;
  const level = Math.min(0.75, Math.max(0.001, velocity * state.volume * sampleGain));

  source.buffer = sample.buffer;
  source.playbackRate.setValueAtTime(playbackRate(sample, note.note), startTime);
  if (sample.loop && sample.mode !== "one-shot") {
    source.loop = true;
    source.loopStart = sample.loop.start;
    source.loopEnd = sample.loop.end;
  }

  gain.gain.setValueAtTime(0.0001, startTime);
  gain.gain.linearRampToValueAtTime(level, startTime + 0.008);
  gain.gain.setValueAtTime(level, Math.max(startTime + 0.01, stopTime - 0.035));
  gain.gain.linearRampToValueAtTime(0.0001, stopTime);
  pan.pan.setValueAtTime(Math.max(-1, Math.min(1, state.pan)), startTime);

  source.connect(gain);
  gain.connect(pan);
  pan.connect(output.master);
  source.start(startTime);
  source.stop(stopTime + 0.05);
  return source;
}

function playbackRate(sample: LoadedSample, note: number): number {
  if (sample.mode === "one-shot") {
    return 1;
  }
  const rootNote = sample.rootNote ?? note;
  return 2 ** ((note - rootNote) / 12);
}
