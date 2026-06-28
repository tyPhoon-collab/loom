import { describe, expect, test } from "vitest";
import { examples } from "./examples";

describe("playground examples", () => {
  test("have unique ids and valid entry files", () => {
    const ids = new Set<string>();

    for (const example of examples) {
      expect(ids.has(example.id), `duplicate example id: ${example.id}`).toBe(false);
      ids.add(example.id);
      expect(example.files.some((file) => file.path === example.entryPath)).toBe(true);
      expect(example.files.some((file) => file.path === example.activePath)).toBe(true);
      expect(example.files.every((file) => file.content.trim().length > 0)).toBe(true);
    }
  });

  test("includes the GM-lite sound preview preset", () => {
    const example = examples.find((candidate) => candidate.id === "gm-lite-sounds");

    expect(example?.files[0]?.content).toContain('title: "GM-lite Sound Preview"');
    expect(example?.files[0]?.content).toContain("# Drums: 10");
    expect(example?.files[0]?.content).toContain("## pc 81");
  });
});
