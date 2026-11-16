import { DIFF_LINES, TRANSCRIPT_ENTRIES, makeDiff, makeTranscript } from "../../src/spike/fixtures";

describe("spike fixtures", () => {
  it("build the 5,000-entry transcript and the 2,000-line diff of the spike", () => {
    expect(makeTranscript()).toHaveLength(TRANSCRIPT_ENTRIES);
    expect(TRANSCRIPT_ENTRIES).toBe(5000);
    expect(makeDiff()).toHaveLength(DIFF_LINES);
    expect(DIFF_LINES).toBe(2000);
  });

  it("are deterministic, with unique keys and all three diff line kinds", () => {
    expect(makeTranscript()).toEqual(makeTranscript());
    expect(new Set(makeTranscript().map((e) => e.id)).size).toBe(TRANSCRIPT_ENTRIES);
    expect(new Set(makeDiff().map((l) => l.kind))).toEqual(new Set(["added", "removed", "context"]));
  });
});
