// A visible keyboard focus ring (FR-019) that does not depend on the host's `:focus-visible`.
import { useState } from "react";
import type { ViewStyle } from "react-native";

export function useFocusRing(color: string): { style: ViewStyle | null; onFocus: () => void; onBlur: () => void } {
  const [focused, setFocused] = useState(false);
  return {
    style: focused ? ({ outlineWidth: 2, outlineStyle: "solid", outlineColor: color, outlineOffset: 2 } as ViewStyle) : null,
    onFocus: () => setFocused(true),
    onBlur: () => setFocused(false),
  };
}
