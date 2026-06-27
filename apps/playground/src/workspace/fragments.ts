import type { PlaygroundFile } from "./types";
import { validateWorkspacePath } from "./zip";

export type NewFragmentInput = {
  manifest: PlaygroundFile;
  existingPaths: Set<string>;
  name: string;
  appendCall: boolean;
};

export type NewFragmentChange = {
  manifestContent: string;
  fragment: PlaygroundFile;
};

export function createFragmentChange(input: NewFragmentInput): NewFragmentChange {
  const name = input.name.trim();
  validateFragmentName(name);

  const path = `sections/${name}.loom`;
  validateWorkspacePath(path);
  if (input.existingPaths.has(path)) {
    throw new Error(`File already exists: ${path}`);
  }

  const manifestContent = addFragmentMapping(input.manifest.content, name, path, input.appendCall);
  return {
    manifestContent,
    fragment: {
      path,
      content: fragmentTemplate(input.manifest.content),
    },
  };
}

function validateFragmentName(name: string): void {
  if (!/^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(name)) {
    throw new Error("Fragment name must start with a letter or number and contain only letters, numbers, `_`, or `-`.");
  }
}

function addFragmentMapping(source: string, name: string, path: string, appendCall: boolean): string {
  const frontmatter = findFrontmatter(source);
  if (!frontmatter) {
    throw new Error("New Fragment requires a manifest with `fragments:` frontmatter.");
  }

  const body = source.slice(frontmatter.bodyStart, frontmatter.bodyEnd);
  const bodyLines = body.split("\n");
  const fragmentsLineIndex = bodyLines.findIndex((line) => line.match(/^fragments:\s*(?:\{\s*\})?\s*$/));
  if (fragmentsLineIndex === -1) {
    throw new Error("New Fragment requires a manifest with `fragments:` frontmatter.");
  }

  if (fragmentMappingExists(bodyLines, fragmentsLineIndex, name)) {
    throw new Error(`Fragment mapping already exists: ${name}`);
  }

  const mappingLine = `  ${name}: ${path}`;
  if (bodyLines[fragmentsLineIndex].match(/^fragments:\s*\{\s*\}\s*$/)) {
    bodyLines.splice(fragmentsLineIndex, 1, "fragments:", mappingLine);
  } else {
    bodyLines.splice(fragmentMappingInsertIndex(bodyLines, fragmentsLineIndex), 0, mappingLine);
  }

  const nextSource = `${source.slice(0, frontmatter.bodyStart)}${bodyLines.join("\n")}${source.slice(frontmatter.bodyEnd)}`;
  if (!appendCall) {
    return nextSource;
  }

  return `${nextSource.replace(/\s*$/, "")}\n\n[[${name}]]\n`;
}

function findFrontmatter(source: string): { bodyStart: number; bodyEnd: number } | null {
  if (!source.startsWith("---\n")) {
    return null;
  }

  const endMarker = source.indexOf("\n---", 4);
  if (endMarker === -1) {
    return null;
  }

  return {
    bodyStart: 4,
    bodyEnd: endMarker,
  };
}

function fragmentMappingExists(lines: string[], fragmentsLineIndex: number, name: string): boolean {
  const mappingPattern = new RegExp(`^\\s+${escapeRegExp(name)}:\\s*`);
  for (let index = fragmentsLineIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === "") {
      continue;
    }
    if (!line.match(/^\s+/)) {
      return false;
    }
    if (mappingPattern.test(line)) {
      return true;
    }
  }
  return false;
}

function fragmentMappingInsertIndex(lines: string[], fragmentsLineIndex: number): number {
  let index = fragmentsLineIndex + 1;
  while (index < lines.length && (lines[index].trim() === "" || lines[index].match(/^\s+/))) {
    index += 1;
  }
  return index;
}

function fragmentTemplate(manifestSource: string): string {
  const channel = firstTrackChannel(manifestSource) ?? "1";
  return `# ${channel}\nC4 | ^ . . . |\n`;
}

function firstTrackChannel(source: string): string | null {
  for (const line of source.split("\n")) {
    const match = line.match(/^#\s+[^@#].*:\s*(\d+)\s*$/);
    if (match) {
      return match[1];
    }
  }
  return null;
}

function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
