import { describe, expect, test } from "vitest";
import sourcesRaw from "../../sounds/gm-lite-sources.json?raw";
import manifestRaw from "../../public/sounds/gm-lite/manifest.json?raw";

describe("GM-lite manifest policy", () => {
  test("uses source-library levels by default", async () => {
    const sources = JSON.parse(sourcesRaw);
    const manifest = JSON.parse(manifestRaw);

    expect(entriesWithGain(sources.instruments)).toEqual([]);
    expect(entriesWithGain(sources.drums)).toEqual([]);
    expect(entriesWithGain(manifest.instruments)).toEqual([]);
    expect(entriesWithGain(manifest.drums)).toEqual([]);
  });

  test("omits default sample loop fields", async () => {
    const manifest = JSON.parse(manifestRaw);

    expect(entriesWithLoop(manifest.instruments)).toEqual([]);
    expect(entriesWithLoop(manifest.drums)).toEqual([]);
  });
});

function entriesWithGain(entries: Record<string, unknown>): string[] {
  return Object.entries(entries)
    .filter(([, entry]) => hasGain(entry))
    .map(([key]) => key);
}

function hasGain(entry: unknown): boolean {
  return typeof entry === "object" && entry !== null && "gain" in entry;
}

function entriesWithLoop(entries: Record<string, unknown>): string[] {
  return Object.entries(entries)
    .filter(([, entry]) => hasLoop(entry))
    .map(([key]) => key);
}

function hasLoop(entry: unknown): boolean {
  return typeof entry === "object" && entry !== null && "loop" in entry;
}
