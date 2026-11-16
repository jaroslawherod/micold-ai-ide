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

/** The iced Overview's grid (research R13): card min-width, gap, most cards per row. */
export const CARD_MIN_WIDTH = 280;
export const CARD_GAP = 16;
export const MAX_COLUMNS = 5;
/** The sidebar's width in the wide arrangement. */
export const SIDEBAR_WIDTH = 300;

/** Cards per row in `width`: `clamp(floor((width + gap) / (min + gap)), 1, 5)`, one when narrow. */
export function gridColumns(width: number, wide: boolean): number {
  if (!wide) return 1;
  return Math.min(MAX_COLUMNS, Math.max(1, Math.floor((width + CARD_GAP) / (CARD_MIN_WIDTH + CARD_GAP))));
}
