import { describe, expect, test } from "vitest";
import { drumForNote } from "./drums";
import { instrumentForProgram } from "./gm-lite-map";

describe("GM-lite mapping", () => {
  test("maps every General MIDI program to a built-in instrument", () => {
    const instruments = new Set(Array.from({ length: 128 }, (_, program) => instrumentForProgram(program)));

    expect(instruments).toEqual(new Set(["piano", "pluck", "pad", "bass", "lead"]));
    expect(instrumentForProgram(-1)).toBe("piano");
    expect(instrumentForProgram(999)).toBe("pad");
  });

  test("maps GM percussion aliases and leaves unknown percussion silent", () => {
    expect(drumForNote(35)).toBe("kick");
    expect(drumForNote(36)).toBe("kick");
    expect(drumForNote(38)).toBe("snare");
    expect(drumForNote(40)).toBe("snare");
    expect(drumForNote(42)).toBe("closed-hat");
    expect(drumForNote(46)).toBe("open-hat");
    expect(drumForNote(45)).toBe("low-tom");
    expect(drumForNote(50)).toBe("high-tom");
    expect(drumForNote(49)).toBe("crash");
    expect(drumForNote(51)).toBe("ride");
    expect(drumForNote(80)).toBeNull();
  });
});
