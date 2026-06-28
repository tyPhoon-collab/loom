export type SampleMode = "one-shot" | "pitched";

export type SampleDefinition = {
  path: string;
  rootNote?: number;
  mode?: SampleMode;
  loop?: {
    start: number;
    end: number;
  } | null;
  gain?: number;
};

export type GmLiteManifest = {
  version: 1;
  instruments: Record<string, SampleDefinition>;
  drums: Record<string, SampleDefinition>;
};

export type LoadedSample = SampleDefinition & {
  key: string;
  buffer: AudioBuffer;
};
