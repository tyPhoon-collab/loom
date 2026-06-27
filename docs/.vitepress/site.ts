const playgroundBaseUrl = process.env.LOOM_PLAYGROUND_URL ?? "http://127.0.0.1:5174/";

export const siteUrls = {
  playground: normalizeBaseUrl(playgroundBaseUrl),
} as const;

function normalizeBaseUrl(url: string): string {
  return url.endsWith("/") ? url : `${url}/`;
}
