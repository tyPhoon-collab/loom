import { describe, expect, test } from "vitest";
import {
  currentFile,
  initModel,
  update,
  type CompileOutput,
  type Effects,
  type FormatOutput,
  type Message,
  type Model,
} from "./model";

function initialModel(): Model {
  return initModel({ hash: "", search: "" })[0];
}

describe("playground model", () => {
  test("source edit marks workspace dirty and clears compile output", () => {
    const compiled = compileOk(initialModel());
    const [next, commands] = update(compiled, {
      type: "source-changed",
      path: compiled.activePath,
      content: "# 1\nD4 | ^ |\n",
    });

    expect(currentFile(next).content).toContain("D4");
    expect(next.compileStatus).toBe("dirty");
    expect(next.dirty).toBe(true);
    expect(next.compiledEvents).toEqual([]);
    expect(next.diagnostics).toEqual([]);
    expect(commands).toHaveLength(1);
  });

  test("compile success stores output", () => {
    const output: CompileOutput = {
      status: "ok",
      events: [{ Note: { time: 0, duration: 1, channel: 0, note: 60, velocity: 100 } }],
      metadata: { bpm: 120, signature: "4/4", unit: "bar", loop: false, loop_range: null },
    };
    const [next, commands] = update(initialModel(), {
      type: "compile-finished",
      output,
      reason: "manual",
    });

    expect(next.compileStatus).toBe("ok");
    expect(next.eventCount).toBe(1);
    expect(next.compiledEvents).toBe(output.events);
    expect(next.metadata).toBe(output.metadata);
    expect(commands).toEqual([]);
  });

  test("format success updates active file and schedules compile", async () => {
    const model = initialModel();
    const output: FormatOutput = { status: "ok", source: "# 1\nC4 | ^ |\n" };
    const [next, commands] = update(model, { type: "format-finished", output });

    expect(currentFile(next).content).toBe(output.source);
    expect(next.dirty).toBe(true);
    expect(await dispatchedBy(commands[0])).toMatchObject([
      { type: "compile-finished", reason: "manual" },
    ]);
  });

  test("set entry marks workspace dirty", () => {
    const model = {
      ...initialModel(),
      files: [
        { path: "song.loom", content: "# 1\nC4 | ^ |\n" },
        { path: "parts/a.loom", content: "# 1\nD4 | ^ |\n" },
      ],
      entryPath: "song.loom",
      activePath: "parts/a.loom",
    };
    const [next, commands] = update(model, { type: "set-entry-requested" });

    expect(next.entryPath).toBe("parts/a.loom");
    expect(next.dirty).toBe(true);
    expect(commands).toHaveLength(1);
  });

  test("play request compiles first when output is not fresh", async () => {
    const [next, commands] = update(initialModel(), { type: "play-requested" });

    expect(next.compileStatus).toBe("loading");
    expect(commands).toHaveLength(2);
    expect(await dispatchedBy(commands[1])).toMatchObject([
      { type: "compile-finished", reason: "play" },
    ]);
  });
});

async function dispatchedBy(command: (effects: Effects, dispatch: (message: Message) => void) => void | Promise<void>) {
  const messages: Message[] = [];
  await command(fakeEffects(), (message) => messages.push(message));
  return messages;
}

function fakeEffects(): Effects {
  return {
    async initWasm() {},
    compile() {
      return {
        status: "ok",
        events: [],
        metadata: { bpm: 120, signature: "4/4", unit: "bar", loop: false, loop_range: null },
      };
    },
    format() {
      return { status: "ok", source: "" };
    },
    prompt() {
      return null;
    },
    confirm() {
      return true;
    },
    async share() {
      return null;
    },
    exportZip() {},
    async importZip() {
      return null;
    },
    async play() {},
    stopPlayback() {},
  };
}

function compileOk(model: Model): Model {
  return update(model, {
    type: "compile-finished",
    reason: "manual",
    output: {
      status: "ok",
      events: [{ Note: { time: 0, duration: 1, channel: 0, note: 60, velocity: 100 } }],
      metadata: { bpm: 120, signature: "4/4", unit: "bar", loop: false, loop_range: null },
    },
  })[0];
}
