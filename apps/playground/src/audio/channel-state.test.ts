import { describe, expect, test } from "vitest";
import type { MidiEvent } from "../compiler/types";
import { initialChannelStates, scheduledNotes } from "./channel-state";

describe("channel state", () => {
  test("applies program, volume, and pan before note scheduling", () => {
    const events: MidiEvent[] = [
      { ProgramChange: { time: 0, channel: 0, program: 32 } },
      { ControlChange: { time: 0, channel: 0, cc: 7, value: 64 } },
      { ControlChange: { time: 0, channel: 0, cc: 10, value: 127 } },
      { Note: { time: 1, duration: 1, channel: 0, note: 48, velocity: 100 } },
    ];

    const [scheduled] = scheduledNotes(events);

    expect(scheduled.state.instrument).toBe("bass");
    expect(scheduled.state.volume).toBeCloseTo(64 / 127);
    expect(scheduled.state.pan).toBeCloseTo(1);
  });

  test("loop initial state includes changes before loop start", () => {
    const events: MidiEvent[] = [
      { ProgramChange: { time: 0, channel: 0, program: 40 } },
      { ControlChange: { time: 0.5, channel: 0, cc: 7, value: 80 } },
      { Note: { time: 4, duration: 1, channel: 0, note: 60, velocity: 100 } },
    ];

    const states = initialChannelStates(events, 4);
    const [scheduled] = scheduledNotes([events[2]], states);

    expect(scheduled.state.instrument).toBe("pad");
    expect(scheduled.state.volume).toBeCloseTo(80 / 127);
  });
});
