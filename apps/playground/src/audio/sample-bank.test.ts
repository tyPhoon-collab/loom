import { afterEach, describe, expect, test, vi } from "vitest";
import { loadInstrumentSample, resetSampleBankForTest } from "./sample-bank";

describe("sample bank", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    resetSampleBankForTest();
  });

  test("caches duplicate decode requests", async () => {
    const decodeAudioData = vi.fn(async () => ({ duration: 1 }) as AudioBuffer);
    const fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/sounds/gm-lite/manifest.json")) {
        return response({
          version: 1,
          instruments: {
            piano: { path: "instruments/piano-c4.wav", rootNote: 60 },
          },
          drums: {},
        });
      }
      return {
        ok: true,
        arrayBuffer: async () => new ArrayBuffer(8),
      } as Response;
    });

    vi.stubGlobal("window", { location: { href: "https://example.test/playground/" } });
    vi.stubGlobal("fetch", fetch);

    const context = { decodeAudioData } as unknown as AudioContext;
    await Promise.all([
      loadInstrumentSample(context, "piano"),
      loadInstrumentSample(context, "piano"),
    ]);

    expect(fetch).toHaveBeenCalledTimes(2);
    expect(decodeAudioData).toHaveBeenCalledTimes(1);
  });
});

function response(body: unknown): Response {
  return {
    ok: true,
    json: async () => body,
  } as Response;
}
