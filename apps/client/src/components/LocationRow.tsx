// A worktree row with its sessions (FR-010, FR-011, FR-019): one line per name, the full text in
// the accessible label, the state indicator first so a long name never moves it.
import React from "react";
import { Pressable, View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import type { RowKey } from "@micold/protocol";
import type { LocationRowModel, SessionRowModel } from "../features/sidebar/store";
import { useT } from "../i18n/IntlProvider";
import { useFocusRing } from "../platform/focusRing";
import { spacing } from "../theme/tokens.generated";
import { StateIndicator } from "./StateIndicator";

function SessionRow({ row }: { row: SessionRowModel }) {
  const t = useT();
  const theme = useTheme();
  const { session } = row;
  return (
    <View
      testID="session-row"
      accessible
      accessibilityLabel={t("sidebar.session", { name: session.name, state: t(`state.${session.state}`) })}
      style={{ flexDirection: "row", alignItems: "center", gap: spacing.sm, paddingStart: spacing.lg }}>
      <StateIndicator state={session.state} />
      <Text variant="bodySmall" numberOfLines={1} style={{ flex: 1, minWidth: 0 }}>
        {session.name}
      </Text>
      <Text variant="labelSmall" style={{ color: theme.colors.onSurfaceVariant, flexShrink: 0 }}>
        {t(session.kind === "terminal" ? "kind.terminal" : "kind.structured")}
      </Text>
    </View>
  );
}

export interface LocationRowProps {
  row: LocationRowModel;
  stale: boolean;
  selected: boolean;
  onSelect: (key: RowKey) => void;
}

function LocationRowView({ row, stale, selected, onSelect }: LocationRowProps) {
  const t = useT();
  const theme = useTheme();
  const ring = useFocusRing(theme.colors.primary);
  const { location } = row;
  const values = { name: location.name, branch: location.branch };
  return (
    <View style={{ opacity: stale ? 0.6 : 1 }}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t(stale ? "sidebar.locationStale" : "sidebar.location", values)}
        accessibilityState={{ selected }}
        focusable
        onFocus={ring.onFocus}
        onBlur={ring.onBlur}
        onPress={() => onSelect(row.key)}
        style={[
          { paddingVertical: spacing.xs, paddingHorizontal: spacing.sm, borderRadius: 8 },
          selected ? { backgroundColor: theme.colors.secondaryContainer } : null,
          ring.style,
        ]}
      >
        <Text testID="location-name" variant="bodyMedium" numberOfLines={1} ellipsizeMode="tail">
          {location.name}
        </Text>
      </Pressable>
      {row.sessions.length === 0 ? (
        <Text variant="bodySmall" style={{ color: theme.colors.onSurfaceVariant, paddingStart: spacing.lg }}>
          {t("sidebar.noSessions")}
        </Text>
      ) : (
        row.sessions.map((s) => <SessionRow key={s.key} row={s} />)
      )}
    </View>
  );
}

/** Rows whose entities are unchanged skip rendering: `applyChanges` keeps their references (SC-002). */
export function sameRow(a: LocationRowProps, b: LocationRowProps): boolean {
  return (
    a.stale === b.stale &&
    a.selected === b.selected &&
    a.onSelect === b.onSelect &&
    a.row.location === b.row.location &&
    a.row.project === b.row.project &&
    a.row.sessions.length === b.row.sessions.length &&
    a.row.sessions.every((s, i) => s.session === b.row.sessions[i]!.session)
  );
}

export const LocationRow = React.memo(LocationRowView, sameRow);
