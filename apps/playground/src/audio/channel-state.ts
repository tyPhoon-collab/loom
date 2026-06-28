import type { MidiEvent, NoteEvent } from "../compiler/types";
import { DEFAULT_INSTRUMENT, instrumentForProgram } from "./gm-lite-map";

export type ChannelState = {
  instrument: string;
  volume: number;
  pan: number;
};

export type ScheduledNote = {
  note: NoteEvent;
  state: ChannelState;
};

export function defaultChannelState(): ChannelState {
  return {
    instrument: DEFAULT_INSTRUMENT,
    volume: 100 / 127,
    pan: 0,
  };
}

export function initialChannelStates(events: MidiEvent[], beforeBeat: number): ChannelState[] {
  const states = createChannelStates();
  for (const event of sortedEvents(events)) {
    const change = eventChange(event);
    if (!change || change.time >= beforeBeat) {
      continue;
    }
    applyChange(states, change);
  }
  return states;
}

export function scheduledNotes(events: MidiEvent[], initialStates?: ChannelState[]): ScheduledNote[] {
  const states = initialStates ? cloneChannelStates(initialStates) : createChannelStates();
  const notes: ScheduledNote[] = [];

  for (const event of sortedEvents(events)) {
    if ("Note" in event) {
      notes.push({
        note: event.Note,
        state: { ...states[channelIndex(event.Note.channel)] },
      });
      continue;
    }

    const change = eventChange(event);
    if (change) {
      applyChange(states, change);
    }
  }

  return notes;
}

function sortedEvents(events: MidiEvent[]): MidiEvent[] {
  return [...events].sort((a, b) => eventTime(a) - eventTime(b) || eventPriority(a) - eventPriority(b));
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

function eventPriority(event: MidiEvent): number {
  return "Note" in event ? 1 : 0;
}

type ChannelChange =
  | { type: "program"; time: number; channel: number; program: number }
  | { type: "control"; time: number; channel: number; cc: number; value: number };

function eventChange(event: MidiEvent): ChannelChange | null {
  if ("ProgramChange" in event) {
    return { type: "program", ...event.ProgramChange };
  }
  if ("ControlChange" in event) {
    return { type: "control", ...event.ControlChange };
  }
  return null;
}

function applyChange(states: ChannelState[], change: ChannelChange): void {
  const state = states[channelIndex(change.channel)];
  if (change.type === "program") {
    state.instrument = instrumentForProgram(change.program);
    return;
  }

  if (change.cc === 7) {
    state.volume = clampMidi(change.value) / 127;
  } else if (change.cc === 10) {
    state.pan = (clampMidi(change.value) - 64) / 63;
  }
}

function createChannelStates(): ChannelState[] {
  return Array.from({ length: 16 }, () => defaultChannelState());
}

function cloneChannelStates(states: ChannelState[]): ChannelState[] {
  return states.map((state) => ({ ...state }));
}

function channelIndex(channel: number): number {
  return Math.max(0, Math.min(15, Math.trunc(channel)));
}

function clampMidi(value: number): number {
  return Math.max(0, Math.min(127, Math.trunc(value)));
}
