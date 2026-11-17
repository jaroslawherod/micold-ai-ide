// A session's state (FR-011): a glyph and the state's name, never colour alone. Working pulses,
// unless the person asked for reduced motion; then the state changes with no animation.
import React, { useEffect, useRef } from "react";
import { Animated, View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { assertNever, type SessionState } from "@micold/protocol";
import { useT } from "../i18n/IntlProvider";
import type { MessageId } from "../i18n/catalog";
import { useReducedMotion } from "../platform/reducedMotion";
import { spacing } from "../theme/tokens.generated";

const GLYPH: Record<SessionState, string> = { working: "●", waiting: "◆", idle: "○", error: "✕", stopped: "■" };

function label(state: SessionState): MessageId {
  switch (state) {
    case "working":
      return "state.working";
    case "waiting":
      return "state.waiting";
    case "idle":
      return "state.idle";
    case "error":
      return "state.error";
    case "stopped":
      return "state.stopped";
    default:
      return assertNever(state);
  }
}

export function StateIndicator({ state }: { state: SessionState }) {
  const t = useT();
  const theme = useTheme();
  const reduced = useReducedMotion();
  const opacity = useRef(new Animated.Value(1)).current;
  const pulse = state === "working" && !reduced;
  useEffect(() => {
    opacity.setValue(1);
    if (!pulse) return;
    const loop = Animated.loop(
      Animated.sequence([
        Animated.timing(opacity, { toValue: 0.4, duration: 600, useNativeDriver: false }),
        Animated.timing(opacity, { toValue: 1, duration: 600, useNativeDriver: false }),
      ]),
    );
    loop.start();
    return () => loop.stop();
  }, [pulse, opacity]);
  const color = state === "error" ? theme.colors.error : state === "waiting" ? theme.colors.tertiary : theme.colors.onSurfaceVariant;
  return (
    <View testID="state-indicator" style={{ flexDirection: "row", alignItems: "center", gap: spacing.xs, flexShrink: 0 }}>
      <Animated.Text testID="state-glyph" importantForAccessibility="no" style={{ color, opacity, width: 16, textAlign: "center" }}>
        {GLYPH[state]}
      </Animated.Text>
      <Text testID="state-text" variant="labelMedium" numberOfLines={1} style={{ color }}>
        {t(label(state))}
      </Text>
    </View>
  );
}
