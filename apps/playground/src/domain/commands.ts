import type { MidiEvent } from "../compiler/types";
import type { PlaygroundFile } from "../workspace/types";
import type { ZipWorkspace } from "../workspace/zip";
import type { Command, CompileReason, PlaybackOptions } from "./types";

export const initWasmCommand: Command = async (effects, dispatch) => {
  try {
    await effects.initWasm();
    dispatch({ type: "compiler-loaded" });
  } catch (error) {
    dispatch({ type: "compiler-load-failed", message: String(error) });
  }
};

export const stopPlaybackCommand: Command = (effects) => {
  effects.stopPlayback();
};

export const promptNewFileCommand: Command = (effects, dispatch) => {
  dispatch({ type: "new-file-submitted", path: effects.prompt("New .loom path", "sections/new.loom") });
};

export const promptFragmentNameCommand: Command = (effects, dispatch) => {
  dispatch({ type: "fragment-name-submitted", name: effects.prompt("New fragment name", "intro") });
};

export const compileCommand =
  (reason: CompileReason, workspace: ZipWorkspace): Command =>
  (effects, dispatch) => {
    dispatch({ type: "compile-finished", output: effects.compile(workspace), reason });
  };

export const formatCommand =
  (source: string): Command =>
  (effects, dispatch) => {
    dispatch({ type: "format-finished", output: effects.format(source) });
  };

export const confirmFragmentAppendCommand =
  (name: string, entryPath: string): Command =>
  (effects, dispatch) => {
    dispatch({
      type: "fragment-append-confirmed",
      name,
      appendCall: effects.confirm(`Add [[${name}]] to the end of ${entryPath}?`),
    });
  };

export const promptRenameCommand =
  (currentPath: string): Command =>
  (effects, dispatch) => {
    dispatch({ type: "rename-submitted", path: effects.prompt("Rename file", currentPath) });
  };

export const confirmDeleteCommand =
  (path: string): Command =>
  (effects, dispatch) => {
    dispatch({ type: "delete-confirmed", confirmed: effects.confirm(`Delete ${path}?`) });
  };

export const confirmLoadExampleCommand =
  (exampleId: string): Command =>
  (effects, dispatch) => {
    dispatch({
      type: "load-example-confirmed",
      exampleId,
      confirmed: effects.confirm("Discard current workspace changes and load this example?"),
    });
  };

export const shareCommand =
  (workspace: ZipWorkspace): Command =>
  async (effects, dispatch) => {
    try {
      const message = await effects.share(workspace);
      if (message) {
        dispatch({ type: "share-finished", message });
      }
    } catch (error) {
      dispatch({ type: "workspace-error", message: error instanceof Error ? error.message : String(error) });
    }
  };

export const exportZipCommand =
  (files: PlaygroundFile[]): Command =>
  (effects, dispatch) => {
    try {
      effects.exportZip(files);
    } catch (error) {
      dispatch({ type: "workspace-error", message: error instanceof Error ? error.message : String(error) });
    }
  };

export const importZipCommand =
  (file: File, dirty: boolean): Command =>
  async (effects, dispatch) => {
    try {
      const workspace = await effects.importZip(file, dirty);
      if (workspace) {
        dispatch({ type: "import-zip-finished", workspace });
      }
    } catch (error) {
      dispatch({ type: "workspace-error", message: error instanceof Error ? error.message : String(error) });
    }
  };

export const playCommand =
  (events: MidiEvent[], options: PlaybackOptions): Command =>
  async (effects, dispatch) => {
    try {
      await effects.play(events, options, dispatch);
    } catch (error) {
      dispatch({ type: "playback-failed", message: String(error) });
    }
  };
