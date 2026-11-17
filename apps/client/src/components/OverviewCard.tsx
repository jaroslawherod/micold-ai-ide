// One Overview card per worktree (FR-012): its project, name, branch and every session's state.
// Each line is one line high, so a state change never resizes the card or moves its neighbours.
import React from "react";
import { View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import type { LocationRowModel } from "../features/sidebar/store";
import { useT } from "../i18n/IntlProvider";
import { spacing } from "../theme/tokens.generated";
import { StateIndicator } from "./StateIndicator";

export function OverviewCard({ row, stale, width }: { row: LocationRowModel; stale: boolean; width: number }) {
  const t = useT();
  const theme = useTheme();
  const { location, project, sessions } = row;
  return (
    <View
      testID="overview-card"
      accessible
      accessibilityLabel={t("overview.card", { project: project.name, name: location.name, sessions: sessions.length })}
      style={{
        width,
        padding: spacing.md,
        gap: spacing.xs,
        borderRadius: 12,
        backgroundColor: theme.colors.surfaceVariant,
        opacity: stale ? 0.6 : 1,
      }}
    >
      <Text variant="labelMedium" numberOfLines={1} style={{ color: theme.colors.onSurfaceVariant }}>
        {project.name}
      </Text>
      <Text variant="titleMedium" numberOfLines={1}>
        {location.name}
      </Text>
      <Text variant="bodySmall" numberOfLines={1} style={{ color: theme.colors.onSurfaceVariant }}>
        {t("overview.branch", { branch: location.branch })}
      </Text>
      {stale ? (
        <Text testID="card-stale" variant="labelSmall" style={{ color: theme.colors.onSurfaceVariant }}>
          {t("overview.stale")}
        </Text>
      ) : null}
      {sessions.map((s) => (
        <View key={s.key} style={{ flexDirection: "row", alignItems: "center", gap: spacing.sm }}>
          <StateIndicator state={s.session.state} />
          <Text variant="bodySmall" numberOfLines={1} style={{ flex: 1, minWidth: 0 }}>
            {s.session.name}
          </Text>
        </View>
      ))}
    </View>
  );
}
