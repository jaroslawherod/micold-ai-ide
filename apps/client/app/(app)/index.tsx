// Home (US2, US3): the sidebar beside the Overview from 600 px, stacked above it below.
import React from "react";
import { ScrollView, View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { LocalePicker } from "../../src/components/LocalePicker";
import { Overview } from "../../src/features/overview/Overview";
import { Sidebar } from "../../src/features/sidebar/Sidebar";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { useT } from "../../src/i18n/IntlProvider";
import { useTitle } from "../../src/i18n/useTitle";
import { SIDEBAR_WIDTH, useLayout } from "../../src/theme/layout";
import { spacing } from "../../src/theme/tokens.generated";

export default function Index() {
  const t = useT();
  const theme = useTheme();
  const { width, wide } = useLayout();
  useTitle("app.title");
  const pad = spacing.lg;
  const header = (
    <View style={{ padding: pad, gap: spacing.sm, flexDirection: wide ? "row" : "column", alignItems: wide ? "center" : "stretch" }}>
      <Text variant="headlineMedium" accessibilityRole="header" style={{ flex: wide ? 1 : undefined }}>
        {t("app.title")}
      </Text>
      <LocalePicker />
    </View>
  );
  const sidebar = (
    <View testID="sidebar-pane" style={wide ? { width: SIDEBAR_WIDTH, flexShrink: 0 } : { alignSelf: "stretch" }}>
      <Sidebar />
    </View>
  );
  const overview = (estimatedWidth: number) => (
    <View testID="overview-pane" style={{ flex: wide ? 1 : undefined, minWidth: 0, padding: pad }}>
      <Overview wide={wide} estimatedWidth={estimatedWidth} />
    </View>
  );
  return (
    <SnapshotProvider>
      <View style={{ flex: 1, backgroundColor: theme.colors.background }}>
        {wide ? (
          <>
            {header}
            <View testID="home-body" style={{ flex: 1, flexDirection: "row" }}>
              <ScrollView style={{ width: SIDEBAR_WIDTH, flexGrow: 0 }}>{sidebar}</ScrollView>
              <ScrollView style={{ flex: 1 }}>{overview(width - SIDEBAR_WIDTH - 2 * pad)}</ScrollView>
            </View>
          </>
        ) : (
          <ScrollView testID="home-scroll" style={{ flex: 1 }}>
            {header}
            <View testID="home-body" style={{ flexDirection: "column" }}>
              {sidebar}
              {overview(width - 2 * pad)}
            </View>
          </ScrollView>
        )}
      </View>
    </SnapshotProvider>
  );
}
