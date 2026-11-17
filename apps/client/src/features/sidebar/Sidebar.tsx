// The sidebar (FR-010, FR-011, FR-013): every daemon's projects, worktrees and sessions, grouped
// by daemon and project in the orchestrator's order. An offline daemon stays listed, marked stale.
import React, { useCallback, useState } from "react";
import { View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { assertNever, type DaemonStatus, type RowKey } from "@micold/protocol";
import { LocationRow } from "../../components/LocationRow";
import { useT, useTranslator } from "../../i18n/IntlProvider";
import type { MessageId } from "../../i18n/catalog";
import { useNow } from "../../platform/useNow";
import { spacing } from "../../theme/tokens.generated";
import { useSnapshot } from "./SnapshotProvider";
import { isStale, sidebarTree, type DaemonGroup } from "./store";

const STATUS_GLYPH: Record<DaemonStatus, string> = { online: "●", offline: "○", "needs-update": "▲" };

export function statusLabel(status: DaemonStatus): MessageId {
  switch (status) {
    case "online":
      return "daemon.status.online";
    case "offline":
      return "daemon.status.offline";
    case "needs-update":
      return "daemon.status.needsUpdate";
    default:
      return assertNever(status);
  }
}

/** A centred message for the loading, failed and empty states. */
export function Notice({ id, alert = false }: { id: MessageId; alert?: boolean }) {
  const t = useT();
  return (
    <View accessible accessibilityRole={alert ? "alert" : "text"} style={{ padding: spacing.md }}>
      <Text variant="bodyMedium">{t(id)}</Text>
    </View>
  );
}

function DaemonSection({ group, selected, onSelect }: { group: DaemonGroup; selected?: RowKey; onSelect: (k: RowKey) => void }) {
  const { t, formatSince } = useTranslator();
  const theme = useTheme();
  const connection = useSnapshot((s) => s.connection);
  const updatedAt = useSnapshot((s) => s.updatedAt);
  const now = useNow();
  const stale = isStale(group.daemon.status, connection);
  return (
    <View style={{ gap: spacing.xs }}>
      <View testID="daemon-header" accessibilityRole="header" style={{ flexDirection: "row", alignItems: "center", gap: spacing.sm }}>
        <Text variant="titleSmall" numberOfLines={1} style={{ flex: 1, minWidth: 0 }}>
          {group.daemon.name}
        </Text>
        <Text variant="labelSmall" style={{ color: theme.colors.onSurfaceVariant, flexShrink: 0 }}>
          {STATUS_GLYPH[group.daemon.status]} {t(statusLabel(group.daemon.status))}
        </Text>
      </View>
      {stale && updatedAt !== null ? (
        <Text testID="stale-note" variant="bodySmall" style={{ color: theme.colors.onSurfaceVariant }}>
          {t("sidebar.stale", { when: formatSince(updatedAt, Math.max(now, updatedAt)) })}
        </Text>
      ) : null}
      {group.projects.length === 0 ? <Notice id="sidebar.noProjects" /> : null}
      {group.projects.map((project) => (
        <View key={project.key} style={{ gap: spacing.xs }}>
          <Text testID="project-header" variant="labelLarge" numberOfLines={1} style={{ color: theme.colors.onSurfaceVariant }}>
            {project.project.name}
          </Text>
          {project.locations.map((row) => (
            <LocationRow key={row.key} row={row} stale={stale} selected={row.key === selected} onSelect={onSelect} />
          ))}
        </View>
      ))}
    </View>
  );
}

export function Sidebar() {
  const t = useT();
  const snapshot = useSnapshot((s) => s.snapshot);
  const failed = useSnapshot((s) => s.failed);
  const [selected, setSelectedState] = useState<RowKey | undefined>(undefined);
  const setSelected = useCallback((key: RowKey) => setSelectedState(key), []);
  let body: React.ReactNode;
  if (failed) body = <Notice id="sidebar.loadFailed" alert />;
  else if (!snapshot) body = <Notice id="sidebar.loading" />;
  else if (snapshot.daemons.length === 0) body = <Notice id="sidebar.empty" />;
  else
    body = sidebarTree(snapshot).map((group) => (
      <DaemonSection key={group.key} group={group} selected={selected} onSelect={setSelected} />
    ));
  return (
    <View testID="sidebar" accessibilityLabel={t("sidebar.label")} role="navigation" style={{ gap: spacing.md, padding: spacing.md }}>
      {body}
    </View>
  );
}
