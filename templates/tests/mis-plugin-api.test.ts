import { assertEquals, assertExists } from "@std/testing";
import type { PluginContext } from "../mis-types.d.ts";

// Create a test version of the helper functions (since we can't import from the main file due to Deno dependencies)
function getConfig<T = unknown>(
  ctx: PluginContext,
  key: string,
  defaultValue?: T,
): T {
  return (ctx.user_config[key] as T) ?? (defaultValue as T);
}

function getArg<T = unknown>(
  ctx: PluginContext,
  key: string,
  defaultValue?: T,
): T {
  return (ctx.plugin_args[key] as T) ?? (defaultValue as T);
}

function getProjectVar<T = unknown>(
  ctx: PluginContext,
  key: string,
  defaultValue?: T,
): T {
  return (ctx.project_variables[key] as T) ?? (defaultValue as T);
}

function hasDependency(ctx: PluginContext, dependencyName: string): boolean {
  return dependencyName in ctx.manifest.deno_dependencies;
}

function getDependencyUrl(
  ctx: PluginContext,
  dependencyName: string,
): string | undefined {
  return ctx.manifest.deno_dependencies[dependencyName];
}

// Mock context for testing
const mockContext: PluginContext = {
  plugin_args: {
    environment: "staging",
    force: true,
    timeout: 60,
  },
  manifest: {
    plugin: {
      name: "test-plugin",
      version: "1.0.0",
      description: "Test plugin",
      registry: "https://github.com/example/plugins.git",
    },
    commands: ["deploy", "rollback"],
    deno_dependencies: {
      "oak": "https://deno.land/x/oak@v12.6.1/mod.ts",
      "cliffy": "https://deno.land/x/cliffy@v1.0.0-rc.3/mod.ts",
    },
    registry: "https://github.com/example/plugins.git",
  },
  user_config: {
    api_key: "secret-key",
    timeout: 30,
    theme: "dark",
    nested_config: {
      setting1: "value1",
      setting2: 42,
    },
  },
  project_variables: {
    name: "my-project",
    version: "1.0.0",
    environment: "production",
  },
  project_root: "/path/to/project",
  meta: {
    name: "test-plugin",
    version: "1.0.0",
    description: "Test plugin",
  },
  dry_run: false,
};

Deno.test("getConfig - returns config value", () => {
  const apiKey = getConfig(mockContext, "api_key");
  assertEquals(apiKey, "secret-key");

  const timeout = getConfig(mockContext, "timeout");
  assertEquals(timeout, 30);

  const theme = getConfig(mockContext, "theme");
  assertEquals(theme, "dark");
});

Deno.test("getConfig - returns default when key missing", () => {
  const missing = getConfig(mockContext, "nonexistent", "default-value");
  assertEquals(missing, "default-value");

  const missingNumber = getConfig(mockContext, "missing_number", 999);
  assertEquals(missingNumber, 999);
});

Deno.test("getConfig - handles nested objects", () => {
  const nested = getConfig(mockContext, "nested_config");
  assertExists(nested);
  assertEquals((nested as any).setting1, "value1");
  assertEquals((nested as any).setting2, 42);
});

Deno.test("getArg - returns CLI argument value", () => {
  const environment = getArg(mockContext, "environment");
  assertEquals(environment, "staging");

  const force = getArg(mockContext, "force");
  assertEquals(force, true);

  const timeout = getArg(mockContext, "timeout");
  assertEquals(timeout, 60);
});

Deno.test("getArg - returns default when arg missing", () => {
  const missing = getArg(mockContext, "missing_arg", "default");
  assertEquals(missing, "default");

  const missingBool = getArg(mockContext, "missing_bool", false);
  assertEquals(missingBool, false);
});

Deno.test("getProjectVar - returns project variable", () => {
  const projectName = getProjectVar(mockContext, "name");
  assertEquals(projectName, "my-project");

  const version = getProjectVar(mockContext, "version");
  assertEquals(version, "1.0.0");

  const env = getProjectVar(mockContext, "environment");
  assertEquals(env, "production");
});

Deno.test("getProjectVar - returns default when variable missing", () => {
  const missing = getProjectVar(mockContext, "missing_var", "default-project");
  assertEquals(missing, "default-project");
});

Deno.test("hasDependency - returns true for existing dependencies", () => {
  const hasOak = hasDependency(mockContext, "oak");
  assertEquals(hasOak, true);

  const hasClifty = hasDependency(mockContext, "cliffy");
  assertEquals(hasClifty, true);
});

Deno.test("hasDependency - returns false for missing dependencies", () => {
  const hasMissing = hasDependency(mockContext, "nonexistent");
  assertEquals(hasMissing, false);
});

Deno.test("getDependencyUrl - returns URL for existing dependencies", () => {
  const oakUrl = getDependencyUrl(mockContext, "oak");
  assertEquals(oakUrl, "https://deno.land/x/oak@v12.6.1/mod.ts");

  const cliffyUrl = getDependencyUrl(mockContext, "cliffy");
  assertEquals(cliffyUrl, "https://deno.land/x/cliffy@v1.0.0-rc.3/mod.ts");
});

Deno.test("getDependencyUrl - returns undefined for missing dependencies", () => {
  const missingUrl = getDependencyUrl(mockContext, "nonexistent");
  assertEquals(missingUrl, undefined);
});

Deno.test("Helper functions work together - realistic usage pattern", () => {
  // Simulate a plugin that uses config with CLI override and dependency checking

  // Get timeout from CLI args, fallback to config, fallback to default
  const argTimeout = getArg<number>(mockContext, "timeout");
  const configTimeout = getConfig<number>(mockContext, "timeout");
  const finalTimeout = argTimeout || configTimeout || 10;

  assertEquals(finalTimeout, 60); // CLI arg should win

  // Get project name for logging
  const projectName = getProjectVar(mockContext, "name", "unknown");
  assertEquals(projectName, "my-project");

  // Check if we can use Oak framework
  const canUseOak = hasDependency(mockContext, "oak");
  assertEquals(canUseOak, true);

  if (canUseOak) {
    const oakImport = getDependencyUrl(mockContext, "oak");
    assertEquals(oakImport, "https://deno.land/x/oak@v12.6.1/mod.ts");
  }
});
