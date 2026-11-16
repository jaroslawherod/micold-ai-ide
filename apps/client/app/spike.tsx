// The spike route (US7 scenario 3): present only in dev and mock builds. Expo inlines the
// environment variable at build time, so in a production export the condition is a constant, Metro
// folds the branch away before it collects dependencies and the spike module never reaches the
// bundle; the route then redirects home. The bundle check scans the export for the mock sentinel
// (`__MICOLD_MOCK__`).
import React from "react";
import { Redirect } from "expo-router";

export default function SpikeRoute() {
  if (process.env.EXPO_PUBLIC_API_MODE === "mock") {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const { SpikeScreen } = require("../src/spike/SpikeScreen") as typeof import("../src/spike/SpikeScreen");
    return <SpikeScreen />;
  }
  return <Redirect href="/" />;
}
