import type { MidiEvent } from "../compiler/types";
import { initialChannelStates, scheduledNotes, type ScheduledNote } from "./channel-state";
import { drumForNote, isPercussionChannel } from "./drums";
import { loadDrumSample, loadInstrumentSample } from "./sample-bank";
import { createPlaybackOutput, scheduleSample } from "./sample-player";

declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext;
  }
}

type Playback = {
  context: AudioContext;
  nodes: AudioBufferSourceNode[];
  timers: number[];
  startedAt: number;
  bpm: number;
  loop: boolean;
  loopStartBeat: number;
  loopEndBeat: number;
};

let playback: Playback | null = null;

export type NotePreviewOptions = {
  bpm: number;
  loop: boolean;
  loopRange?: {
    startBeat: number;
    endBeat: number;
  };
};

export type NotePreviewPosition = {
  beat: number;
  seconds: number;
  loop: boolean;
};

export function isNotePreviewPlaying(): boolean {
  return playback !== null;
}

export async function playNotePreview(
  events: MidiEvent[],
  options: NotePreviewOptions,
  onEnded: () => void,
): Promise<void> {
  stopNotePreview();

  const AudioContextConstructor = window.AudioContext ?? window.webkitAudioContext;
  if (!AudioContextConstructor) {
    throw new Error("WebAudio is not supported in this browser.");
  }

  const context = new AudioContextConstructor();
  try {
    await context.resume();

    const bpm = Math.max(1, options.bpm);
    const secondsPerBeat = 60 / bpm;
    const startOffset = 0.05;
    const nodes: AudioBufferSourceNode[] = [];
    const timers: number[] = [];
    const noteEvents = events.flatMap((event) => ("Note" in event ? [event.Note] : []));
    const sequenceEndBeat = Math.max(...noteEvents.map((note) => note.time + note.duration), 1);
    const loopStartBeat = options.loopRange?.startBeat ?? 0;
    const loopEndBeat = options.loopRange?.endBeat ?? sequenceEndBeat;
    const loopLengthSeconds = Math.max(0.1, (loopEndBeat - loopStartBeat) * secondsPerBeat);
    const notes = options.loop
      ? scheduledNotes(
          events.filter((event) => eventTime(event) >= loopStartBeat && eventTime(event) < loopEndBeat),
          initialChannelStates(events, loopStartBeat),
        )
      : scheduledNotes(events);

    if (options.loop && notes.length === 0) {
      throw new Error("Nothing to loop: no note events are inside the loop range.");
    }

    await loadSamplesForNotes(context, notes);
    const output = createPlaybackOutput(context);
    const now = context.currentTime;

    playback = {
      context,
      nodes,
      timers,
      startedAt: performance.now(),
      bpm,
      loop: options.loop,
      loopStartBeat,
      loopEndBeat,
    };

    if (options.loop) {
      let iteration = 0;
      const scheduleLoop = async () => {
        if (!playback) {
          return;
        }
        await scheduleNotes(context, output, nodes, notes, now + startOffset + iteration * loopLengthSeconds, secondsPerBeat, loopStartBeat);
        iteration += 1;
        timers.length = 0;
        timers.push(window.setTimeout(() => void scheduleLoop(), loopLengthSeconds * 1000));
      };
      await scheduleLoop();
      return;
    }

    await scheduleNotes(context, output, nodes, notes, now + startOffset, secondsPerBeat, 0);
    const endTime = now + startOffset + sequenceEndBeat * secondsPerBeat;
    timers.push(
      window.setTimeout(() => {
        playback = null;
        void context.close();
        onEnded();
      }, Math.max(0, (endTime - now + 0.1) * 1000)),
    );
  } catch (error) {
    void context.close();
    playback = null;
    throw error;
  }
}

export function notePreviewPosition(): NotePreviewPosition | null {
  if (!playback) {
    return null;
  }

  const seconds = Math.max(0, (performance.now() - playback.startedAt) / 1000);
  const elapsedBeats = seconds * (playback.bpm / 60);
  if (!playback.loop) {
    return {
      beat: elapsedBeats,
      seconds,
      loop: false,
    };
  }

  const loopLength = playback.loopEndBeat - playback.loopStartBeat;
  return {
    beat: playback.loopStartBeat + (elapsedBeats % loopLength),
    seconds,
    loop: true,
  };
}

export function stopNotePreview(): void {
  if (!playback) {
    return;
  }

  for (const timer of playback.timers) {
    window.clearTimeout(timer);
  }
  for (const node of playback.nodes) {
    try {
      node.stop();
    } catch {
      // Sources may already have stopped naturally.
    }
  }
  void playback.context.close();
  playback = null;
}

async function loadSamplesForNotes(context: AudioContext, notes: ScheduledNote[]): Promise<void> {
  await Promise.all(notes.map((note) => sampleForNote(context, note)));
}

async function scheduleNotes(
  context: AudioContext,
  output: ReturnType<typeof createPlaybackOutput>,
  nodes: AudioBufferSourceNode[],
  notes: ScheduledNote[],
  anchorTime: number,
  secondsPerBeat: number,
  beatOffset: number,
): Promise<void> {
  for (const note of notes) {
    const sample = await sampleForNote(context, note);
    if (!sample) {
      continue;
    }

    const startTime = anchorTime + (note.note.time - beatOffset) * secondsPerBeat;
    const source = scheduleSample(context, output, sample, note.note, note.state, startTime, secondsPerBeat);
    source.addEventListener("ended", () => {
      const index = nodes.indexOf(source);
      if (index !== -1) {
        nodes.splice(index, 1);
      }
    });
    nodes.push(source);
  }
}

async function sampleForNote(context: AudioContext, note: ScheduledNote) {
  if (isPercussionChannel(note.note.channel)) {
    const drum = drumForNote(note.note.note);
    return drum ? loadDrumSample(context, drum) : null;
  }
  return loadInstrumentSample(context, note.state.instrument);
}

function eventTime(event: MidiEvent): number {
  if ("Note" in event) {
    return event.Note.time;
  }
  if ("ProgramChange" in event) {
    return event.ProgramChange.time;
  }
  return event.ControlChange.time;
}
