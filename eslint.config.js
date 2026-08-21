import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import prettier from "eslint-config-prettier";

export default tseslint.config(
  {
    ignores: ["dist", "coverage", "src-tauri/target", "src-tauri/gen", "node_modules"],
  },

  js.configs.recommended,

  // Type-aware linting, scoped to TypeScript only. These rules are the reason
  // typescript-eslint is here: the app is full of async IPC, and an un-awaited
  // promise is a silent bug that no other tool catches.
  {
    files: ["**/*.{ts,tsx}"],
    extends: [...tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      ecmaVersion: 2023,
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      // Unused args are fine when prefixed with _, which keeps signatures
      // readable when implementing an interface.
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],
    },
  },

  // Config files run in Node and are outside the app's type graph, so the
  // type-aware rules cannot apply to them.
  {
    files: ["**/*.js", "**/*.cjs", "**/*.mjs"],
    extends: [tseslint.configs.disableTypeChecked],
    languageOptions: { globals: globals.node },
  },

  // Vite and Vitest configs are TypeScript but execute in Node.
  {
    files: ["vite.config.ts", "vitest.config.ts"],
    languageOptions: { globals: globals.node },
  },

  // Prettier last: turns off every stylistic rule it would otherwise fight.
  prettier,
);
