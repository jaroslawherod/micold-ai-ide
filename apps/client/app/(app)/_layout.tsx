// Every route in this group needs a session (FR-008); sign-in and the dev spike sit outside it.
import React from "react";
import { Slot } from "expo-router";
import { AuthGate } from "../../src/auth/AuthGate";

export default function ProtectedLayout() {
  return (
    <AuthGate>
      <Slot />
    </AuthGate>
  );
}
