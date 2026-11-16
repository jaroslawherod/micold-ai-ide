// Sign-in (US1, FR-007): only the providers the auth service reports as configured, a "not set up"
// state when there are none, readable refusal and cancellation with a retry, and a return to the
// route the person asked for. All text comes from the catalog.
import React, { useCallback, useEffect, useState } from "react";
import { View } from "react-native";
import { ActivityIndicator, Button, Text, useTheme } from "react-native-paper";
import { useLocalSearchParams, useRouter } from "expo-router";
import { isAuthFailure, type AuthProvider } from "@micold/protocol";
import { useApi } from "../src/api/ApiProvider";
import { returnTarget } from "../src/auth/guard";
import { useSession } from "../src/auth/SessionProvider";
import { LocalePicker } from "../src/components/LocalePicker";
import { useT } from "../src/i18n/IntlProvider";
import { useTitle } from "../src/i18n/useTitle";
import { spacing } from "../src/theme/tokens.generated";

type Providers = AuthProvider[] | "loading" | "failed";
type Failure = { reason: "refused" | "cancelled"; provider: AuthProvider } | null;

export default function SignIn() {
  const t = useT();
  const theme = useTheme();
  const { auth } = useApi();
  const router = useRouter();
  const { returnTo } = useLocalSearchParams<{ returnTo?: string }>();
  const setSession = useSession((s) => s.setSession);
  const [providers, setProviders] = useState<Providers>("loading");
  const [failure, setFailure] = useState<Failure>(null);
  const [busy, setBusy] = useState(false);
  useTitle("signin.title");

  const load = useCallback(() => {
    setProviders("loading");
    auth.listProviders().then(
      (all) => setProviders(all.filter((p) => p.configured)),
      () => setProviders("failed"),
    );
  }, [auth]);
  useEffect(load, [load]);

  const signIn = async (provider: AuthProvider) => {
    setBusy(true);
    setFailure(null);
    const result = await auth.signIn(provider.id);
    setBusy(false);
    if (isAuthFailure(result)) {
      setFailure({ reason: result.failure, provider });
      return;
    }
    setSession(result);
    router.replace(returnTarget(returnTo) as never);
  };

  return (
    <View style={{ flex: 1, padding: spacing.lg, gap: spacing.md, backgroundColor: theme.colors.background }}>
      <Text variant="headlineMedium" accessibilityRole="header">
        {t("signin.title")}
      </Text>
      {providers === "loading" && <ActivityIndicator accessibilityLabel={t("signin.loading")} />}
      {providers === "failed" && (
        <>
          <Text accessibilityRole="alert">{t("signin.loadFailed")}</Text>
          <Button mode="outlined" onPress={load} accessibilityLabel={t("signin.retry")}>
            {t("signin.retry")}
          </Button>
        </>
      )}
      {Array.isArray(providers) && providers.length === 0 && <Text>{t("signin.notSetUp")}</Text>}
      {Array.isArray(providers) &&
        providers.map((p) => (
          <Button
            key={p.id}
            mode="contained"
            disabled={busy}
            onPress={() => void signIn(p)}
            accessibilityLabel={t("signin.with", { provider: p.name })}
          >
            {t("signin.with", { provider: p.name })}
          </Button>
        ))}
      {failure && (
        <View style={{ gap: spacing.sm }}>
          <Text accessibilityRole="alert" style={{ color: theme.colors.error }}>
            {t(failure.reason === "refused" ? "signin.refused" : "signin.cancelled", { provider: failure.provider.name })}
          </Text>
          <Button mode="outlined" onPress={() => void signIn(failure.provider)} accessibilityLabel={t("signin.retry")}>
            {t("signin.retry")}
          </Button>
        </View>
      )}
      <LocalePicker />
    </View>
  );
}
