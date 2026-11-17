// The person's reduced-motion setting (Edge Cases, Reduced motion): `prefers-reduced-motion` on the
// web through AccessibilityInfo. Unknown or failing counts as reduced, so nothing animates by mistake.
import { useEffect, useState } from "react";
import { AccessibilityInfo } from "react-native";

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(true);
  useEffect(() => {
    let live = true;
    AccessibilityInfo.isReduceMotionEnabled().then(
      (on) => live && setReduced(on),
      () => undefined,
    );
    const sub = AccessibilityInfo.addEventListener("reduceMotionChanged", (on) => setReduced(on));
    return () => {
      live = false;
      sub?.remove();
    };
  }, []);
  return reduced;
}
