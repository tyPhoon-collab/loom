export const DEFAULT_INSTRUMENT = "piano";

export function instrumentForProgram(program: number): string {
  const value = clampProgram(program);
  if (value <= 7) {
    return "piano";
  }
  if (value <= 15) {
    return "pluck";
  }
  if (value <= 23) {
    return "pad";
  }
  if (value <= 31) {
    return "pluck";
  }
  if (value <= 39) {
    return "bass";
  }
  if (value <= 55) {
    return "pad";
  }
  if (value <= 63) {
    return "lead";
  }
  if (value <= 79) {
    return "lead";
  }
  if (value <= 95) {
    return "lead";
  }
  return "pad";
}

function clampProgram(program: number): number {
  if (!Number.isFinite(program)) {
    return 0;
  }
  return Math.max(0, Math.min(127, Math.trunc(program)));
}
