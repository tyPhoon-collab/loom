import type { GmLiteManifest, LoadedSample, SampleDefinition } from "./types";

const MANIFEST_PATH = "sounds/gm-lite/manifest.json";

let manifestPromise: Promise<GmLiteManifest> | null = null;
const samplePromises = new Map<string, Promise<LoadedSample>>();

export async function loadManifest(): Promise<GmLiteManifest> {
  manifestPromise ??= fetchJson<GmLiteManifest>(assetUrl(MANIFEST_PATH));
  return manifestPromise;
}

export async function loadInstrumentSample(context: AudioContext, key: string): Promise<LoadedSample> {
  const manifest = await loadManifest();
  const sample = manifest.instruments[key];
  if (!sample) {
    throw new Error(`Missing GM-lite instrument sample: ${key}`);
  }
  return loadSample(context, `instrument:${key}`, sample);
}

export async function loadDrumSample(context: AudioContext, key: string): Promise<LoadedSample> {
  const manifest = await loadManifest();
  const sample = manifest.drums[key];
  if (!sample) {
    throw new Error(`Missing GM-lite drum sample: ${key}`);
  }
  return loadSample(context, `drum:${key}`, sample);
}

export function resetSampleBankForTest(): void {
  manifestPromise = null;
  samplePromises.clear();
}

async function loadSample(
  context: AudioContext,
  key: string,
  sample: SampleDefinition,
): Promise<LoadedSample> {
  const cacheKey = `${key}:${sample.path}`;
  let promise = samplePromises.get(cacheKey);
  if (!promise) {
    promise = fetch(sampleUrl(sample.path))
      .then(async (response) => {
        if (!response.ok) {
          throw new Error(`Failed to load sample ${sample.path}: ${response.status}`);
        }
        return response.arrayBuffer();
      })
      .then((buffer) => context.decodeAudioData(buffer.slice(0)))
      .then((audioBuffer) => ({ ...sample, key, buffer: audioBuffer }));
    samplePromises.set(cacheKey, promise);
  }
  return promise;
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load GM-lite sample manifest: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

function sampleUrl(path: string): string {
  return assetUrl(`sounds/gm-lite/${path}`);
}

function assetUrl(path: string): string {
  return new URL(path, baseUrl()).toString();
}

function baseUrl(): string {
  return new URL(import.meta.env.BASE_URL, window.location.href).toString();
}
