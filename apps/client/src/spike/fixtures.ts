// Fixed fixtures of the spike (R12, SC-008): a 5,000-entry transcript and a 2,000-line diff.
export const TRANSCRIPT_ENTRIES = 5000;
export const DIFF_LINES = 2000;

export interface TranscriptEntry {
  id: string;
  role: "user" | "assistant" | "tool";
  text: string;
}

export interface DiffLine {
  id: string;
  kind: "added" | "removed" | "context";
  text: string;
}

const ROLES = ["user", "assistant", "tool"] as const;

export function makeTranscript(count = TRANSCRIPT_ENTRIES): TranscriptEntry[] {
  return Array.from({ length: count }, (_, i) => ({
    id: `entry-${i}`,
    role: ROLES[i % ROLES.length] as TranscriptEntry["role"],
    text: `Entry ${i}: ${"the quick brown fox jumps over the lazy dog ".repeat(1 + (i % 4))}`,
  }));
}

export function makeDiff(count = DIFF_LINES): DiffLine[] {
  return Array.from({ length: count }, (_, i) => {
    const kind = i % 5 === 1 ? "removed" : i % 5 === 2 ? "added" : "context";
    const sign = kind === "added" ? "+" : kind === "removed" ? "-" : " ";
    return { id: `line-${i}`, kind, text: `${sign} fn item_${i}(value: u32) -> u32 { value + ${i} }` };
  });
}
