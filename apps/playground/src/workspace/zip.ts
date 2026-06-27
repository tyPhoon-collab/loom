import { strFromU8, strToU8, unzipSync, zipSync } from "fflate";
import type { PlaygroundFile } from "./types";

export type ZipWorkspace = {
  entryPath: string;
  activePath: string;
  files: PlaygroundFile[];
};

export function exportWorkspaceZip(files: PlaygroundFile[]): Uint8Array {
  return zipSync(
    Object.fromEntries(files.map((file) => [file.path, strToU8(file.content)])),
    { level: 6 },
  );
}

export function importWorkspaceZip(data: Uint8Array): ZipWorkspace {
  const unzipped = unzipSync(data);
  const files = Object.entries(unzipped)
    .filter(([path]) => path.endsWith(".loom") && !isHiddenPath(path))
    .map(([path, content]) => {
      const normalizedPath = normalizeZipPath(path);
      validateWorkspacePath(normalizedPath);
      return {
        path: normalizedPath,
        content: strFromU8(content),
      };
    })
    .sort((left, right) => left.path.localeCompare(right.path));

  if (files.length === 0) {
    throw new Error("ZIP does not contain any .loom files.");
  }

  const seen = new Set<string>();
  for (const file of files) {
    if (seen.has(file.path)) {
      throw new Error(`Duplicate workspace path in ZIP: ${file.path}`);
    }
    seen.add(file.path);
  }

  const entryPath = pickDefaultEntryPath(files);
  return {
    entryPath,
    activePath: entryPath,
    files,
  };
}

export function validateWorkspacePath(path: string): void {
  if (
    path.length === 0 ||
    path.startsWith("/") ||
    path.includes("\\") ||
    path.split("/").some((part) => part === "." || part === ".." || part === "")
  ) {
    throw new Error("Use a relative path with `/` separators and no `.` or `..` segments.");
  }
}

function normalizeZipPath(path: string): string {
  return path.replace(/^\.\/+/, "");
}

function isHiddenPath(path: string): boolean {
  return path.split("/").some((part) => part.startsWith(".") || part === "__MACOSX");
}

function pickDefaultEntryPath(files: PlaygroundFile[]): string {
  return files.find((file) => file.path === "song.loom")?.path ?? files[0].path;
}
