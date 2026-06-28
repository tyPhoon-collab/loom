const DRUMS = new Map<number, string>([
  [35, "kick"],
  [36, "kick"],
  [38, "snare"],
  [40, "snare"],
  [42, "closed-hat"],
  [44, "closed-hat"],
  [46, "open-hat"],
  [45, "low-tom"],
  [47, "low-tom"],
  [48, "high-tom"],
  [50, "high-tom"],
  [49, "crash"],
  [57, "crash"],
  [51, "ride"],
]);

export function drumForNote(note: number): string | null {
  return DRUMS.get(note) ?? null;
}

export function isPercussionChannel(channel: number): boolean {
  return channel === 9;
}
