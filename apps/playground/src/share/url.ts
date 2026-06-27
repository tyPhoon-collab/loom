import { gzipSync, gunzipSync, strFromU8, strToU8 } from "fflate";
import { validateWorkspacePath, type ZipWorkspace } from "../workspace/zip";

const HASH_PREFIX = "#loom=";
const PAYLOAD_VERSION = 1;

type SharePayload = {
  version: typeof PAYLOAD_VERSION;
  entryPath: string;
  activePath: string;
  files: Record<string, string>;
};

export type ShareUrl = {
  url: string;
  payloadLength: number;
};

export function createShareUrl(workspace: ZipWorkspace, location: Location): ShareUrl {
  validateWorkspace(workspace);
  const payload: SharePayload = {
    version: PAYLOAD_VERSION,
    entryPath: workspace.entryPath,
    activePath: workspace.activePath,
    files: Object.fromEntries(workspace.files.map((file) => [file.path, file.content])),
  };
  const encodedPayload = encodePayload(JSON.stringify(payload));
  const baseUrl = `${location.origin}${location.pathname}${location.search}`;

  return {
    url: `${baseUrl}${HASH_PREFIX}${encodedPayload}`,
    payloadLength: encodedPayload.length,
  };
}

export function restoreWorkspaceFromHash(hash: string): ZipWorkspace | null {
  if (!hash.startsWith(HASH_PREFIX)) {
    return null;
  }

  const payload = JSON.parse(decodePayload(hash.slice(HASH_PREFIX.length))) as SharePayload;
  if (payload.version !== PAYLOAD_VERSION) {
    throw new Error(`Unsupported share payload version: ${payload.version}`);
  }

  const files = Object.entries(payload.files)
    .map(([path, content]) => ({ path, content }))
    .sort((left, right) => left.path.localeCompare(right.path));
  const workspace = {
    entryPath: payload.entryPath,
    activePath: payload.activePath,
    files,
  };
  validateWorkspace(workspace);
  return workspace;
}

function validateWorkspace(workspace: ZipWorkspace): void {
  if (workspace.files.length === 0) {
    throw new Error("Shared workspace does not contain any files.");
  }

  const seen = new Set<string>();
  for (const file of workspace.files) {
    validateWorkspacePath(file.path);
    if (!file.path.endsWith(".loom")) {
      throw new Error(`Shared workspace file must be a .loom file: ${file.path}`);
    }
    if (seen.has(file.path)) {
      throw new Error(`Duplicate shared workspace path: ${file.path}`);
    }
    seen.add(file.path);
  }

  if (!seen.has(workspace.entryPath)) {
    throw new Error(`Shared workspace entry file is missing: ${workspace.entryPath}`);
  }
  if (!seen.has(workspace.activePath)) {
    throw new Error(`Shared workspace active file is missing: ${workspace.activePath}`);
  }
}

function encodePayload(input: string): string {
  return bytesToBase64Url(gzipSync(strToU8(input), { level: 9 }));
}

function decodePayload(input: string): string {
  return strFromU8(gunzipSync(base64UrlToBytes(input)));
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function base64UrlToBytes(input: string): Uint8Array {
  const padded = input.replaceAll("-", "+").replaceAll("_", "/").padEnd(
    Math.ceil(input.length / 4) * 4,
    "=",
  );
  return Uint8Array.from(atob(padded), (char) => char.charCodeAt(0));
}
