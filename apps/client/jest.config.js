// Jest for the web client (#1735), two projects:
//  - `web` (`jest-expo/web`): react-native-web in jsdom, for logic that touches `window`,
//    `matchMedia` and `navigator` (platform wrappers, i18n, theme).
//  - `components` (`jest-expo`): native host components, which Testing Library queries; the
//    react-native-web DOM host is not supported by its renderer.
// Coverage goes to lcov for scripts/diff-coverage.py.
const transformIgnorePatterns = [
  "node_modules/(?!\\.pnpm/|((jest-)?react-native|@react-native(-community)?|expo(nent)?|@expo(nent)?/.*|@expo-google-fonts/.*|react-navigation|@react-navigation/.*|@sentry/react-native|native-base|react-native-svg|react-native-paper|react-native-safe-area-context|@formatjs/.*|intl-messageformat|zustand))",
];
const shared = {
  rootDir: __dirname,
  moduleNameMapper: { "\\.ttf$": "<rootDir>/tests/support/fileStub.js" },
  transformIgnorePatterns,
};

module.exports = {
  collectCoverageFrom: [
    "src/**/*.{ts,tsx}",
    "app/**/*.{ts,tsx}",
    "!src/theme/tokens.generated.ts",
    "!**/*.d.ts",
  ],
  coverageDirectory: "<rootDir>/coverage",
  coverageReporters: ["lcov", "text-summary"],
  projects: [
    {
      ...shared,
      displayName: "web",
      preset: "jest-expo/web",
      testMatch: ["<rootDir>/tests/**/*.test.ts"],
    },
    {
      ...shared,
      displayName: "components",
      preset: "jest-expo",
      setupFiles: ["<rootDir>/tests/support/nativeSetup.js"],
      testMatch: ["<rootDir>/tests/**/*.test.tsx"],
    },
  ],
};
