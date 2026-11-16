import React from "react";
import { View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { LocalePicker } from "../../src/components/LocalePicker";
import { useT } from "../../src/i18n/IntlProvider";
import { useTitle } from "../../src/i18n/useTitle";
import { spacing } from "../../src/theme/tokens.generated";

export default function Index() {
  const t = useT();
  const theme = useTheme();
  useTitle("app.title");
  return (
    <View style={{ flex: 1, padding: spacing.lg, gap: spacing.md, backgroundColor: theme.colors.background }}>
      <Text variant="headlineMedium" accessibilityRole="header">
        {t("app.title")}
      </Text>
      <Text variant="bodyLarge">{t("app.tagline")}</Text>
      <LocalePicker />
    </View>
  );
}
