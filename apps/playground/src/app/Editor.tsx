import { Compartment, EditorState, RangeSetBuilder, type Extension } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import { basicSetup } from "codemirror";
import { useEffect, useRef } from "preact/hooks";
import type { Diagnostic, Dispatch } from "../domain/model";
import type { PlaygroundFile } from "../workspace/types";

type Props = {
  file: PlaygroundFile;
  diagnostics: Diagnostic[];
  pendingCursor?: {
    path: string;
    line: number;
    column: number;
  };
  dispatch: Dispatch;
};

export function Editor({ file, diagnostics, pendingCursor, dispatch }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const diagnosticCompartment = useRef(new Compartment());
  const ignoreChange = useRef(false);

  useEffect(() => {
    if (!host.current) {
      return;
    }

    const editor = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: file.content,
        extensions: [
          basicSetup,
          EditorView.lineWrapping,
          diagnosticCompartment.current.of(diagnosticDecorations(file.content, diagnostics, file.path)),
          EditorView.updateListener.of((update) => {
            handleEditorUpdate(update, file.path, dispatch, ignoreChange);
          }),
        ],
      }),
    });
    view.current = editor;

    return () => {
      editor.destroy();
      view.current = null;
    };
  }, [file.path, dispatch]);

  useEffect(() => {
    const editor = view.current;
    if (!editor) {
      return;
    }

    const current = editor.state.doc.toString();
    if (current !== file.content) {
      ignoreChange.current = true;
      editor.dispatch({
        changes: { from: 0, to: current.length, insert: file.content },
      });
      ignoreChange.current = false;
    }

    editor.dispatch({
      effects: diagnosticCompartment.current.reconfigure(
        diagnosticDecorations(file.content, diagnostics, file.path),
      ),
    });
  }, [file.content, file.path, diagnostics]);

  useEffect(() => {
    const editor = view.current;
    if (!editor || !pendingCursor || pendingCursor.path !== file.path) {
      return;
    }

    const offset = offsetForLineColumn(
      editor.state.doc.toString(),
      pendingCursor.line,
      pendingCursor.column,
    );
    editor.dispatch({
      selection: { anchor: offset },
      effects: EditorView.scrollIntoView(offset, { y: "center" }),
    });
    editor.focus();
    dispatch({ type: "pending-cursor-applied" });
  }, [dispatch, file.path, pendingCursor]);

  return <div class="editor-host" ref={host} aria-label={`${file.path} source`} />;
}

function handleEditorUpdate(
  update: ViewUpdate,
  path: string,
  dispatch: Dispatch,
  ignoreChange: { current: boolean },
): void {
  if (!update.docChanged || ignoreChange.current) {
    return;
  }

  dispatch({ type: "source-changed", path, content: update.state.doc.toString() });
}

function diagnosticDecorations(source: string, diagnostics: Diagnostic[], activePath: string): Extension {
  const activeDiagnostics = diagnostics.filter(
    (diagnostic) => diagnostic.path === activePath && diagnostic.line,
  );
  const ranges = activeDiagnostics
    .map((diagnostic) => diagnosticRange(source, diagnostic))
    .filter((range): range is { lineStart: number; from: number; to: number } => range !== null)
    .sort((left, right) => left.lineStart - right.lineStart || left.from - right.from);
  const builder = new RangeSetBuilder<Decoration>();

  for (const range of ranges) {
    builder.add(range.lineStart, range.lineStart, Decoration.line({ class: "cm-diagnostic-line" }));
    builder.add(range.from, range.to, Decoration.mark({ class: "cm-diagnostic-mark" }));
  }

  return EditorView.decorations.of(builder.finish() as DecorationSet);
}

function diagnosticRange(
  source: string,
  diagnostic: Diagnostic,
): { lineStart: number; from: number; to: number } | null {
  if (!diagnostic.line) {
    return null;
  }

  const lineStart = offsetForLineColumn(source, diagnostic.line, 1);
  const from = offsetForLineColumn(source, diagnostic.line, diagnostic.column ?? 1);
  const lineEndIndex = source.indexOf("\n", lineStart);
  const lineEnd = lineEndIndex === -1 ? source.length : lineEndIndex;
  const to = Math.min(lineEnd, Math.max(from + 1, from + Math.max(1, diagnostic.length)));

  return {
    lineStart,
    from: Math.min(from, source.length),
    to: Math.min(to, source.length),
  };
}

function offsetForLineColumn(source: string, line: number, column: number): number {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < source.length) {
    const nextNewline = source.indexOf("\n", offset);
    if (nextNewline === -1) {
      return source.length;
    }
    offset = nextNewline + 1;
    currentLine += 1;
  }

  const lineEnd = source.indexOf("\n", offset);
  const end = lineEnd === -1 ? source.length : lineEnd;
  const chars = Array.from(source.slice(offset, end));
  return offset + chars.slice(0, Math.max(0, column - 1)).join("").length;
}
