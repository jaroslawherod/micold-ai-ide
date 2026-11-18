// Shows the connection state (FR-009, FR-014): "connecting" on the first open, the attempt count
// while reconnecting, and, once the retries are spent, the unreachable state with a Retry action.
// Nothing while connected. Announced politely to assistive technology.
import React, { useSyncExternalStore } from "react";
import { View } from "react-native";
import { Button, Text, useTheme } from "react-native-paper";
import { assertNever, type ConnectionState } from "@micold/protocol";
import { useApi } from "../api/ApiProvider";
import { useT } from "../i18n/IntlProvider";
import { spacing } from "../theme/tokens.generated";

export function ConnectionBanner() {
  const t = useT();
  const theme = useTheme();
  const orchestrator = useApi().orchestrator;
  const { connection } = orchestrator;
  const state: ConnectionState = useSyncExternalStore(connection.subscribe, connection.get, connection.get);
  let text: string;
  let detail: string | null = null;
  switch (state.kind) {
    case "connected":
      return null;
    case "connecting":
      text = t("connection.connecting");
      break;
    case "reconnecting":
      text = t("connection.reconnecting", { attempt: state.attempt });
      break;
    case "offline":
      text = t("connection.unreachable.title");
      detail = t("connection.unreachable.body");
      break;
    default:
      return assertNever(state);
  }
  return (
    <View
      testID="connection-banner"
      style={{
        paddingVertical: spacing.sm,
        paddingHorizontal: spacing.md,
        gap: spacing.xs,
        flexDirection: "row",
        flexWrap: "wrap",
        alignItems: "center",
        backgroundColor: theme.colors.errorContainer,
      }}
    >
      <View accessible accessibilityRole="alert" accessibilityLiveRegion="polite" style={{ flex: 1, minWidth: 0 }}>
        <Text style={{ color: theme.colors.onErrorContainer }}>{text}</Text>
        {detail ? <Text variant="bodySmall" style={{ color: theme.colors.onErrorContainer }}>{detail}</Text> : null}
      </View>
      {state.kind === "offline" ? (
        <Button mode="outlined" compact onPress={() => orchestrator.retry()} accessibilityLabel={t("connection.retry")}>
          {t("connection.retry")}
        </Button>
      ) : null}
    </View>
  );
}
