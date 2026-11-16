// The Overview grid (FR-012, research R13): one card per worktree in snapshot order, the iced
// app's column rule on wide screens, one column below 600 px. Cards wrap; nothing scrolls sideways.
import React, { useState } from "react";
import { View, type LayoutChangeEvent } from "react-native";
import { Text } from "react-native-paper";
import { OverviewCard } from "../../components/OverviewCard";
import { useT } from "../../i18n/IntlProvider";
import { CARD_GAP, gridColumns } from "../../theme/layout";
import { spacing } from "../../theme/tokens.generated";
import { Notice } from "../sidebar/Sidebar";
import { useSnapshot } from "../sidebar/SnapshotProvider";
import { isStale, sidebarTree } from "../sidebar/store";

export function Overview({ wide, estimatedWidth }: { wide: boolean; estimatedWidth: number }) {
  const t = useT();
  const snapshot = useSnapshot((s) => s.snapshot);
  const connection = useSnapshot((s) => s.connection);
  const [measured, setMeasured] = useState<number | null>(null);
  const width = Math.max(0, measured ?? estimatedWidth);
  const columns = gridColumns(width, wide);
  const cardWidth = columns === 1 ? width : (width - CARD_GAP * (columns - 1)) / columns;
  const cards = snapshot
    ? sidebarTree(snapshot).flatMap((d) => {
        const stale = isStale(d.daemon.status, connection);
        return d.projects.flatMap((p) => p.locations.map((row) => ({ row, stale })));
      })
    : [];
  return (
    <View style={{ gap: spacing.sm }}>
      <Text variant="titleLarge" accessibilityRole="header">
        {t("overview.title")}
      </Text>
      {snapshot && cards.length === 0 ? <Notice id="overview.empty" /> : null}
      <View
        testID="overview-grid"
        onLayout={(e: LayoutChangeEvent) => setMeasured(e.nativeEvent.layout.width)}
        style={{ flexDirection: "row", flexWrap: "wrap", gap: CARD_GAP }}
      >
        {cards.map(({ row, stale }) => (
          <OverviewCard key={row.key} row={row} stale={stale} width={cardWidth} />
        ))}
      </View>
    </View>
  );
}
