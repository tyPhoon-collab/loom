import type { NoteEvent } from "../compiler/types";

declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext;
  }
}

type Playback = {
  context: AudioContext;
  nodes: OscillatorNode[];
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
  notes: NoteEvent[],
  options: NotePreviewOptions,
  onEnded: () => void,
): Promise<void> {
  stopNotePreview();

  const AudioContextConstructor = window.AudioContext ?? window.webkitAudioContext;
  if (!AudioContextConstructor) {
    throw new Error("WebAudio is not supported in this browser.");
  }

  const context = new AudioContextConstructor();
  await context.resume();

  const bpm = Math.max(1, options.bpm);
  const secondsPerBeat = 60 / bpm;
  const startOffset = 0.05;
  const now = context.currentTime;
  const nodes: OscillatorNode[] = [];
  const timers: number[] = [];
  const sequenceEndBeat = Math.max(...notes.map((note) => note.time + note.duration), 1);
  const loopStartBeat = options.loopRange?.startBeat ?? 0;
  const loopEndBeat = options.loopRange?.endBeat ?? sequenceEndBeat;
  const loopLengthSeconds = Math.max(0.1, (loopEndBeat - loopStartBeat) * secondsPerBeat);
  const loopNotes = notes.filter((note) => note.time >= loopStartBeat && note.time < loopEndBeat);

  if (options.loop && loopNotes.length === 0) {
    void context.close();
    throw new Error("Nothing to loop: no note events are inside the loop range.");
  }

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
    const scheduleLoop = () => {
      if (!playback) {
        return;
      }
      scheduleNotes(context, nodes, loopNotes, now + startOffset + iteration * loopLengthSeconds, secondsPerBeat, loopStartBeat);
      iteration += 1;
      timers.length = 0;
      timers.push(window.setTimeout(scheduleLoop, loopLengthSeconds * 1000));
    };
    scheduleLoop();
    return;
  }

  scheduleNotes(context, nodes, notes, now + startOffset, secondsPerBeat, 0);
  const endTime = now + startOffset + sequenceEndBeat * secondsPerBeat;
  timers.push(
    window.setTimeout(() => {
      playback = null;
      void context.close();
      onEnded();
    }, Math.max(0, (endTime - now + 0.1) * 1000)),
  );
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
      // Oscillators may already have stopped naturally.
    }
  }
  void playback.context.close();
  playback = null;
}

function scheduleNotes(
  context: AudioContext,
  nodes: OscillatorNode[],
  notes: NoteEvent[],
  anchorTime: number,
  secondsPerBeat: number,
  beatOffset: number,
): void {
  for (const note of notes) {
    const startTime = anchorTime + (note.time - beatOffset) * secondsPerBeat;
    const duration = Math.max(0.04, note.duration * secondsPerBeat);
    const stopTime = startTime + duration;
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    const level = Math.max(0.02, Math.min(0.28, (note.velocity / 127) * 0.22));

    oscillator.type = note.channel === 9 ? "triangle" : "sine";
    oscillator.frequency.setValueAtTime(noteFrequency(note.note), startTime);
    gain.gain.setValueAtTime(0.0001, startTime);
    gain.gain.linearRampToValueAtTime(level, startTime + 0.01);
    gain.gain.setValueAtTime(level, Math.max(startTime + 0.01, stopTime - 0.03));
    gain.gain.linearRampToValueAtTime(0.0001, stopTime);

    oscillator.connect(gain);
    gain.connect(context.destination);
    oscillator.addEventListener("ended", () => {
      const index = nodes.indexOf(oscillator);
      if (index !== -1) {
        nodes.splice(index, 1);
      }
    });
    oscillator.start(startTime);
    oscillator.stop(stopTime + 0.02);
    nodes.push(oscillator);
  }
}

function noteFrequency(note: number): number {
  return 440 * 2 ** ((note - 69) / 12);
}
