# TypeScript Templates Test Suite

This directory contains tests for the core TypeScript API and types used by Make
It So plugins.

## What's Tested

- **Types** (`mis-types.test.ts`): Validates that all TypeScript types
  (PluginContext, PluginResult, PluginManifest, etc.) have the correct structure
- **Helper Functions** (`mis-plugin-api.test.ts`): Tests the utility functions
  like `getConfig()`, `getArg()`, `hasDependency()`, etc.
- **JSON Extraction** (`json-extraction.test.ts`): Tests the logic that extracts
  JSON results from mixed plugin output

## Running Tests

```bash
# From the templates/tests directory
cd templates/tests

# Run all tests
deno task test

# Run tests in watch mode (reruns on file changes)
deno task test:watch

# Run a specific test file
deno test mis-types.test.ts --allow-read --allow-env --allow-run

# Run tests with verbose output
deno test --allow-read --allow-env --allow-run --reporter=verbose
```

## Test Structure

### Types Tests

- Validates that all type definitions compile correctly
- Tests that mock objects conform to expected interfaces
- Ensures the new split config structure works properly

### API Function Tests

- Tests helper functions with realistic mock data
- Validates default value behavior
- Tests edge cases (missing keys, undefined values)

### JSON Extraction Tests

- Tests parsing plugin output with mixed content (debug logs + JSON)
- Validates multi-line JSON handling
- Tests error cases (no JSON, malformed JSON)

## Mock Data

The tests use realistic mock data that mirrors what plugins would receive in
production:

```typescript
const mockContext: PluginContext = {
  plugin_args: { environment: "staging", force: true },
  manifest: {
    plugin: { name: "test-plugin", version: "1.0.0" },
    commands: ["deploy", "rollback"],
    deno_dependencies: { oak: "https://deno.land/x/oak@v12.6.1/mod.ts" },
  },
  config: { api_key: "secret", timeout: 30 },
  project_variables: { name: "my-project" },
  // ... etc
};
```

## Continuous Integration

These tests should be run as part of the CI pipeline to ensure:

1. TypeScript types remain compatible
2. Helper functions work correctly
3. Plugin output parsing is robust

## Adding New Tests

When adding new functionality to the TypeScript API:

1. Add type tests for any new interfaces
2. Add function tests for new helper functions
3. Update mock data if new fields are added to PluginContext
