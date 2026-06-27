import type { MidiEvent } from "../compiler/types";
import type { PlaygroundFile } from "../workspace/types";
import type { ZipWorkspace } from "../workspace/zip";

export type Diagnostic = {
  path?: string | null;
  line?: number | null;
  column?: number | null;
  byte_offset?: number | null;
  length: number;
  severity: "error" | "warning";
  message: string;
  help?: string | null;
};

export type CompileOutput =
  | {
      status: "ok";
      events: MidiEvent[];
      metadata: PlaygroundMetadata;
    }
  | {
      status: "err";
      diagnostics: Diagnostic[];
    };

export type FormatOutput =
  | {
      status: "ok";
      source: string;
    }
  | {
      status: "err";
      diagnostics: Diagnostic[];
    };

export type CompileStatus = "loading" | "ready" | "dirty" | "ok" | "err";

export type PlaygroundMetadata = {
  bpm: number;
  signature: string;
  unit: string;
  loop: boolean;
  loop_range?: string | null;
};

export type PlaybackPosition = {
  beat: number;
  seconds: number;
  loop: boolean;
};

export type Model = {
  files: PlaygroundFile[];
  entryPath: string;
  activePath: string;
  diagnostics: Diagnostic[];
  compileStatus: CompileStatus;
  eventCount: number;
  compiledEvents: MidiEvent[];
  metadata?: PlaygroundMetadata;
  isPlaying: boolean;
  playbackPosition?: PlaybackPosition;
  currentExampleId: string;
  dirty: boolean;
  pendingCursor?: {
    path: string;
    line: number;
    column: number;
  };
};

export type CompileReason = "manual" | "boot" | "play" | "load-workspace";

type CompilerMessage =
  | { type: "compiler-loaded" }
  | { type: "compiler-load-failed"; message: string }
  | { type: "compile-requested"; reason: CompileReason }
  | { type: "compile-finished"; output: CompileOutput; reason: CompileReason }
  | { type: "format-requested" }
  | { type: "format-finished"; output: FormatOutput };

type EditorMessage =
  | { type: "source-changed"; path: string; content: string }
  | { type: "file-selected"; path: string }
  | { type: "diagnostic-selected"; index: number }
  | { type: "pending-cursor-applied" };

type FileMessage =
  | { type: "new-file-requested" }
  | { type: "new-file-submitted"; path: string | null }
  | { type: "fragment-requested" }
  | { type: "fragment-name-submitted"; name: string | null }
  | { type: "fragment-append-confirmed"; name: string; appendCall: boolean }
  | { type: "rename-requested" }
  | { type: "rename-submitted"; path: string | null }
  | { type: "delete-requested" }
  | { type: "delete-confirmed"; confirmed: boolean }
  | { type: "set-entry-requested" };

type WorkspaceMessage =
  | { type: "load-example-requested"; exampleId: string }
  | { type: "load-example-confirmed"; exampleId: string; confirmed: boolean }
  | { type: "share-requested" }
  | { type: "share-finished"; message: string }
  | { type: "export-zip-requested" }
  | { type: "import-zip-selected"; file: File }
  | { type: "import-zip-finished"; workspace: ZipWorkspace }
  | { type: "workspace-error"; message: string };

type PlaybackMessage =
  | { type: "play-requested" }
  | { type: "playback-started" }
  | { type: "playback-ended" }
  | { type: "playback-failed"; message: string }
  | { type: "stop-requested" }
  | { type: "playback-tick"; position?: PlaybackPosition };

export type Message =
  | CompilerMessage
  | EditorMessage
  | FileMessage
  | WorkspaceMessage
  | PlaybackMessage;

export type Dispatch = (message: Message) => void;

export type Command = (effects: Effects, dispatch: Dispatch) => void | Promise<void>;

export type Effects = {
  initWasm(): Promise<void>;
  compile(workspace: ZipWorkspace): CompileOutput;
  format(source: string): FormatOutput;
  prompt(message: string, initial?: string): string | null;
  confirm(message: string): boolean;
  share(workspace: ZipWorkspace): Promise<string | null>;
  exportZip(files: PlaygroundFile[]): void;
  importZip(file: File, dirty: boolean): Promise<ZipWorkspace | null>;
  play(events: MidiEvent[], options: PlaybackOptions, dispatch: Dispatch): Promise<void>;
  stopPlayback(): void;
};

export type PlaybackOptions = {
  bpm: number;
  loop: boolean;
  loopRange?: {
    startBeat: number;
    endBeat: number;
  };
};

export type UpdateResult = readonly [Model, readonly Command[]];
