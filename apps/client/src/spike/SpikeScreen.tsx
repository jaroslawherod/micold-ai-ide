/* eslint-disable i18next/no-literal-string -- dev-only spike (R12): this module is in no production export */
// The spike of #1735 (US7, FR-020): render a long transcript and a diff with Paper `Text` and
// `FlatList` on the web and measure. The measured numbers are recorded in specs/1735-.../spike.md.
// `__MICOLD_MOCK__` is the sentinel the bundle check looks for in a production export.
import React, { useCallback, useRef, useState } from "react";
import { FlatList, ScrollView, View } from "react-native";
import { Button, Text, useTheme } from "react-native-paper";
import { DIFF_LINES, TRANSCRIPT_ENTRIES, makeDiff, makeTranscript, type DiffLine, type TranscriptEntry } from "./fixtures";
import type { AppTheme } from "../theme/buildTheme";

export const SPIKE_SENTINEL = "__MICOLD_MOCK__";

type Surface = "transcript" | "diff";

function heapMb(): string {
  const memory = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory;
  return memory ? `${(memory.usedJSHeapSize / 1048576).toFixed(1)} MB heap` : "heap n/a";
}

export function SpikeScreen() {
  const theme = useTheme() as AppTheme;
  const [shown, setShown] = useState<Surface | undefined>();
  const [results, setResults] = useState<Partial<Record<Surface, string>>>({});
  const started = useRef(0);
  const data = useRef<{ transcript?: TranscriptEntry[]; diff?: DiffLine[] }>({});

  const run = (surface: Surface) => {
    data.current[surface] ??= (surface === "transcript" ? makeTranscript() : makeDiff()) as never;
    started.current = performance.now();
    setShown(surface);
  };

  // `onLayout` of the list fires after the first commit and layout: the time since the press is the
  // render time of the surface (frame cost of the first paint).
  const done = useCallback(
    (surface: Surface) => () => {
      const ms = (performance.now() - started.current).toFixed(1);
      const count = surface === "transcript" ? TRANSCRIPT_ENTRIES : DIFF_LINES;
      setResults((r) => ({ ...r, [surface]: `${surface}: ${ms} ms for ${count} rows, ${heapMb()}` }));
    },
    [],
  );

  return (
    <View testID="spike-screen" style={{ flex: 1, backgroundColor: theme.colors.background }}>
      <Text variant="titleMedium">{SPIKE_SENTINEL} spike</Text>
      <View style={{ flexDirection: "row", gap: 8 }}>
        <Button mode="contained" testID="spike-run-transcript" onPress={() => run("transcript")}>
          Render transcript
        </Button>
        <Button mode="contained" testID="spike-run-diff" onPress={() => run("diff")}>
          Render diff
        </Button>
      </View>
      {(["transcript", "diff"] as const).map((s) =>
        results[s] ? (
          <Text key={s} testID={`spike-result-${s}`}>
            {results[s]}
          </Text>
        ) : null,
      )}
      {shown === "transcript" && (
        <FlatList
          testID="spike-list-transcript"
          data={data.current.transcript}
          keyExtractor={(e) => e.id}
          onLayout={done("transcript")}
          initialNumToRender={30}
          renderItem={({ item }) => (
            <Text selectable variant="bodyMedium">
              {item.role}: {item.text}
            </Text>
          )}
        />
      )}
      {shown === "diff" && (
        <ScrollView testID="spike-list-diff" onLayout={done("diff")}>
          {data.current.diff?.map((line) => (
            <Text
              key={line.id}
              selectable
              style={{
                fontFamily: "FiraMono_400Regular",
                backgroundColor:
                  line.kind === "added" ? theme.extraColors.diffAdded : line.kind === "removed" ? theme.extraColors.diffRemoved : undefined,
              }}
            >
              {line.text}
            </Text>
          ))}
        </ScrollView>
      )}
    </View>
  );
}
