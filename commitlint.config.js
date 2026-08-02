/**
 * Commitlint config for File Atlas.
 * Enforces Conventional Commits with a File Atlas-specific type list.
 * See docs/CONTRIBUTING.md for the type reference.
 */
export default {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "type-enum": [
      2,
      "always",
      [
        "feat",
        "fix",
        "perf",
        "refactor",
        "docs",
        "test",
        "chore",
        "ci",
        "security",
        "revert",
        "build",
        "style",
      ],
    ],
    "subject-case": [2, "never", ["pascal-case", "upper-case", "start-case"]],
    "subject-empty": [2, "never"],
    "subject-max-length": [2, "always", 72],
    "type-empty": [2, "never"],
    "body-leading-blank": [2, "always"],
    "footer-leading-blank": [2, "always"],
  },
};
