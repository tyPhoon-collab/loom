import {
  notePreviewPosition,
  playNotePreview,
  stopNotePreview,
} from "../audio/web-audio";
import type { MidiEvent } from "../compiler/types";
import initWasm, {
  compileWorkspace,
  formatFile,
} from "../generated/loom_wasm/loom.js";
import { createShareUrl } from "../share/url";
import type {
  CompileOutput,
  Dispatch,
  Effects,
  FormatOutput,
  PlaybackOptions,
  PlaybackPosition,
} from "../domain/model";
import type { PlaygroundFile } from "../workspace/types";
import { exportWorkspaceZip, importWorkspaceZip, type ZipWorkspace } from "../workspace/zip";

let playbackStatusTimer: number | null = null;

export const playgroundEffects: Effects = {
  async initWasm() {
    await initWasm();
  },
  compile(workspace) {
    return JSON.parse(
      compileWorkspace(
        JSON.stringify({
          entry_path: workspace.entryPath,
          active_path: workspace.activePath,
          files: Object.fromEntries(workspace.files.map((file) => [file.path, file.content])),
        }),
      ),
    ) as CompileOutput;
  },
  format(source) {
    return JSON.parse(formatFile(source)) as FormatOutput;
  },
  prompt(message, initial) {
    return window.prompt(message, initial);
  },
  confirm(message) {
    return window.confirm(message);
  },
  async share(workspace) {
    const shareUrl = createShareUrl(workspace, window.location);
    if (!confirmSharePayloadSize(shareUrl.payloadLength)) {
      return null;
    }

    await copyText(shareUrl.url);
    return `Share URL copied. Payload: ${formatBytes(shareUrl.payloadLength)}.`;
  },
  exportZip(files) {
    exportZip(files);
  },
  async importZip(file, dirty) {
    return importZip(file, dirty);
  },
  async play(events, options, dispatch) {
    await play(events, options, dispatch);
  },
  stopPlayback,
};

function exportZip(files: PlaygroundFile[]): void {
  const data = exportWorkspaceZip(files);
  const buffer = new ArrayBuffer(data.byteLength);
  new Uint8Array(buffer).set(data);
  const url = URL.createObjectURL(new Blob([buffer], { type: "application/zip" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = "loom-playground.zip";
  link.click();
  URL.revokeObjectURL(url);
}

async function importZip(file: File, dirty: boolean): Promise<ZipWorkspace | null> {
  if (dirty && !window.confirm("Discard current workspace changes and import this ZIP?")) {
    return null;
  }

  const workspace = importWorkspaceZip(new Uint8Array(await file.arrayBuffer()));
  const entryPath = chooseEntryPath(workspace);
  return { ...workspace, entryPath, activePath: entryPath };
}

async function play(
  events: MidiEvent[],
  options: PlaybackOptions,
  dispatch: Dispatch,
): Promise<void> {
  dispatch({ type: "playback-loading" });
  await playNotePreview(events, options, () => {
    stopPlaybackStatusTimer();
    dispatch({ type: "playback-ended" });
  });
  dispatch({ type: "playback-started" });
  startPlaybackStatusTimer(dispatch);
}

function stopPlayback(): void {
  stopNotePreview();
  stopPlaybackStatusTimer();
}

function chooseEntryPath(workspace: ZipWorkspace): string {
  if (workspace.files.some((file) => file.path === "song.loom")) {
    return "song.loom";
  }

  if (workspace.files.length === 1) {
    return workspace.entryPath;
  }

  const candidateText = workspace.files.map((file) => file.path).join("\n");
  const selectedPath = window.prompt(
    `Choose entry file from imported workspace:\n\n${candidateText}`,
    workspace.entryPath,
  );
  if (!selectedPath) {
    throw new Error("ZIP import cancelled: no entry file selected.");
  }
  if (!workspace.files.some((file) => file.path === selectedPath)) {
    throw new Error(`Entry file is not in imported ZIP: ${selectedPath}`);
  }
  return selectedPath;
}

function startPlaybackStatusTimer(dispatch: Dispatch): void {
  stopPlaybackStatusTimer();
  const update = () => {
    dispatch({ type: "playback-tick", position: notePreviewPosition() as PlaybackPosition | undefined });
  };
  update();
  playbackStatusTimer = window.setInterval(update, 250);
}

function stopPlaybackStatusTimer(): void {
  if (playbackStatusTimer === null) {
    return;
  }
  window.clearInterval(playbackStatusTimer);
  playbackStatusTimer = null;
}

function confirmSharePayloadSize(payloadLength: number): boolean {
  if (payloadLength > 32 * 1024) {
    throw new Error(`Share URL is too large (${formatBytes(payloadLength)}). Use Export ZIP instead.`);
  }

  if (payloadLength > 16 * 1024) {
    return window.confirm(
      `This share URL is large (${formatBytes(payloadLength)}) and may break in chats or issue trackers. Copy it anyway?`,
    );
  }

  if (payloadLength > 8 * 1024) {
    return window.confirm(
      `This share URL is ${formatBytes(payloadLength)}. ZIP export is safer for larger workspaces. Copy it anyway?`,
    );
  }

  return true;
}

async function copyText(text: string): Promise<void> {
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(text);
    return;
  }
  window.prompt("Copy share URL", text);
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} bytes`;
}
