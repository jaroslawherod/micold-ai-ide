// An opened session's structured content (FR-014b): a snapshot read when the session is opened, with
// a translated refresh action; nothing streams. Loading, empty, failed (translated by error code)
// and truncated entries each have their own line.
import React, { useCallback, useEffect, useState } from "react";
import { View } from "react-native";
import { Button, Text, useTheme } from "react-native-paper";
import { assertNever, type ContentEntry, type SessionId } from "@micold/protocol";
import { useApi } from "../../api/ApiProvider";
import { useT } from "../../i18n/IntlProvider";
import type { MessageId } from "../../i18n/catalog";
import { spacing } from "../../theme/tokens.generated";
import { useSnapshot } from "../sidebar/SnapshotProvider";
import { errorKeyOf } from "./errors";

type Load = { status: "loading" } | { status: "ready"; entries: ContentEntry[] } | { status: "failed"; reason: MessageId };

const ROLE: Record<ContentEntry["role"], MessageId> = {
  user: "session.role.user",
  assistant: "session.role.assistant",
  system: "session.role.system",
};

function kindLabel(kind: ContentEntry["kind"]): MessageId | null {
  switch (kind) {
    case "text":
      return null;
    case "tool_call":
      return "session.entry.toolCall";
    case "tool_result":
      return "session.entry.toolResult";
    case "status":
      return "session.entry.status";
    default:
      return assertNever(kind);
  }
}

function Entry({ entry }: { entry: ContentEntry }) {
  const t = useT();
  const theme = useTheme();
  const kind = kindLabel(entry.kind);
  return (
    <View testID="content-entry" accessible style={{ gap: spacing.xs }}>
      <Text variant="labelSmall" style={{ color: theme.colors.onSurfaceVariant }}>
        {t(ROLE[entry.role])}
        {kind ? ` · ${t(kind)}` : ""}
      </Text>
      <Text variant="bodyMedium" selectable>
        {entry.text}
      </Text>
      {entry.truncated ? (
        <Text variant="bodySmall" style={{ color: theme.colors.onSurfaceVariant }}>
          {t("session.content.truncated")}
        </Text>
      ) : null}
    </View>
  );
}

export function SessionContent({ sessionId }: { sessionId: SessionId }) {
  const t = useT();
  const theme = useTheme();
  const { orchestrator } = useApi();
  const snapshot = useSnapshot((s) => s.snapshot);
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const session = snapshot?.sessions.find((s) => s.id === sessionId);
  const location = snapshot?.locations.find((l) => l.id === session?.locationId);
  const project = snapshot?.projects.find((p) => p.id === location?.projectId);
  const daemon = project?.daemonId;
  const projectId = project?.id;

  const read = useCallback(() => {
    if (!daemon || !projectId) return () => undefined;
    let current = true;
    setLoad({ status: "loading" });
    orchestrator.readSession({ daemon, project: projectId, session: sessionId }).then(
      (entries) => current && setLoad({ status: "ready", entries }),
      (err: unknown) => current && setLoad({ status: "failed", reason: errorKeyOf(err) }),
    );
    return () => {
      current = false;
    };
  }, [orchestrator, daemon, projectId, sessionId]);
  useEffect(read, [read]);

  return (
    <View testID="session-content" accessibilityLabel={t("session.content.label")} role="region" style={{ gap: spacing.md }}>
      <View style={{ flexDirection: "row", alignItems: "center", gap: spacing.sm }}>
        <Text variant="titleMedium" accessibilityRole="header" numberOfLines={1} style={{ flex: 1, minWidth: 0 }}>
          {session?.name ?? ""}
        </Text>
        {session?.reachable === false ? (
          <Text testID="session-unreachable" variant="labelMedium" style={{ color: theme.colors.error }}>
            {t("session.unreachable")}
          </Text>
        ) : null}
        <Button mode="outlined" compact onPress={read} accessibilityLabel={t("session.refresh")}>
          {t("session.refresh")}
        </Button>
      </View>
      {load.status === "loading" ? <Text>{t("session.content.loading")}</Text> : null}
      {load.status === "failed" ? (
        <View accessible accessibilityRole="alert" style={{ gap: spacing.xs }}>
          <Text>{t("session.content.failed")}</Text>
          <Text variant="bodySmall">{t(load.reason)}</Text>
        </View>
      ) : null}
      {load.status === "ready" && load.entries.length === 0 ? <Text>{t("session.content.empty")}</Text> : null}
      {load.status === "ready" ? load.entries.map((e) => <Entry key={e.seq} entry={e} />) : null}
    </View>
  );
}
