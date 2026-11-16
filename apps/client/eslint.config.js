// Lint for the web client (#1735): a11y rules (FR-019), no user-visible literal (FR-014, SC-004),
// logical start/end layout only (FR-016, R14).
const js = require("@eslint/js");
const tseslint = require("typescript-eslint");
const i18next = require("eslint-plugin-i18next");
const a11y = require("eslint-plugin-react-native-a11y");
const hooks = require("eslint-plugin-react-hooks");
const globals = require("globals");

const PHYSICAL =
  "^(left|right|marginLeft|marginRight|paddingLeft|paddingRight|borderLeft.*|borderRight.*)$";
const physicalLayout = [
  {
    selector: `Property[key.name=/${PHYSICAL}/]`,
    message: "Use logical start/end layout (start, end, marginStart, paddingEnd, ...), never left/right (FR-016).",
  },
  {
    selector: `Property[key.value=/${PHYSICAL}/]`,
    message: "Use logical start/end layout (start, end, marginStart, paddingEnd, ...), never left/right (FR-016).",
  },
  {
    selector: "Property[key.name='textAlign'][value.value=/^(left|right)$/]",
    message: "Use textAlign 'start'/'end' (FR-016).",
  },
];

module.exports = tseslint.config(
  { ignores: ["dist/**", "dist-mock/**", "coverage/**", ".expo/**", "node_modules/**", "src/theme/tokens.generated.ts", "*.config.js"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}", "app/**/*.{ts,tsx}"],
    plugins: { i18next, "react-native-a11y": a11y, "react-hooks": hooks },
    rules: {
      ...a11y.configs.all.rules,
      // A hint is optional; the rule would demand one beside every label (FR-019 asks for names).
      "react-native-a11y/has-accessibility-hint": "off",
      "react-hooks/rules-of-hooks": "error",
      "i18next/no-literal-string": ["error", { mode: "jsx-text-only" }],
      "no-restricted-syntax": ["error", ...physicalLayout],
    },
  },
  {
    // Jest mocks are hoisted CommonJS; the support files are plain Node scripts.
    files: ["tests/**/*.{ts,tsx,js}"],
    languageOptions: { globals: { ...globals.node, ...globals.jest } },
    rules: { "@typescript-eslint/no-require-imports": "off" },
  },
);
