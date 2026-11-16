// Logical (start/end) layout helpers (FR-016): a right-to-left locale mirrors with no code change
// because no style in the Client names left or right (lint rule, research R14).
import { useWindowDimensions, type ViewStyle } from "react-native";

/** The one breakpoint: from this width a screen uses its wide arrangement. */
export const BREAKPOINT = 600;

export const isWide = (width: number): boolean => width >= BREAKPOINT;

export function useLayout(): { width: number; wide: boolean } {
  const { width } = useWindowDimensions();
  return { width, wide: isWide(width) };
}

export const insetStart = (value: number): ViewStyle => ({ start: value });
export const marginInline = (value: number): ViewStyle => ({ marginStart: value, marginEnd: value });
export const paddingInline = (value: number): ViewStyle => ({ paddingStart: value, paddingEnd: value });
