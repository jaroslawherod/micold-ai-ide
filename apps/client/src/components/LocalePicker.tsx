// The language control (FR-015, FR-019): one radio per available locale, named by the catalog,
// operable with the keyboard (each radio is a focusable pressable), switching the locale at runtime
// without a reload. Colours come from the theme, spacing and corners from the token scales.
import React from "react";
import { Pressable, StyleSheet, View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { useT } from "../i18n/IntlProvider";
import { useLocaleStore } from "../i18n/localeStore";
import type { MessageId } from "../i18n/catalog";
import { shape, spacing } from "../theme/tokens.generated";

export function LocalePicker() {
  const t = useT();
  const theme = useTheme();
  const locale = useLocaleStore((s) => s.locale);
  const available = useLocaleStore((s) => s.available);
  const setLocale = useLocaleStore((s) => s.setLocale);
  return (
    <View
      accessible
      accessibilityRole="radiogroup"
      accessibilityLabel={t("locale.picker.label")}
      style={styles.group}
    >
      {available.map((code) => {
        const checked = code === locale;
        const label = t(`locale.name.${code}` as MessageId);
        return (
          <Pressable
            key={code}
            accessibilityRole="radio"
            accessibilityLabel={label}
            accessibilityState={{ checked }}
            aria-checked={checked}
            onPress={() => setLocale(code)}
            testID={`locale-${code}`}
            style={[
              styles.option,
              {
                backgroundColor: checked ? theme.colors.secondaryContainer : "transparent",
                borderColor: theme.colors.outline,
              },
            ]}
          >
            <Text
              variant="labelLarge"
              style={{ color: checked ? theme.colors.onSecondaryContainer : theme.colors.onSurface }}
            >
              {label}
            </Text>
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  group: { flexDirection: "row", flexWrap: "wrap", gap: spacing.sm },
  option: {
    borderWidth: 1,
    borderRadius: shape.small,
    paddingVertical: spacing.sm,
    paddingHorizontal: spacing.md,
    minHeight: 40,
    justifyContent: "center",
  },
});
