import { assertEquals, assertExists } from "@std/testing";

// Import the types from the parent directory
import type {
  PluginContext,
  PluginManifest,
  PluginMeta,
  PluginResult,
} from "../mis-types.d.ts";

Deno.test("PluginContext - has required fields", () => {
  const mockContext: PluginContext = {
    plugin_args: { environment: "staging", force: true },
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
      },
      registry: "https://github.com/example/plugins.git",
    },
    config: {
      api_key: "secret",
      timeout: 30,
      theme: "dark",
    },
    project_variables: {
      name: "my-project",
      version: "1.0.0",
    },
    project_root: "/path/to/project",
    meta: {
      name: "test-plugin",
      version: "1.0.0",
      description: "Test plugin",
      registry: "https://github.com/example/plugins.git",
    },
    dry_run: false,
  };

  // Verify all required fields exist
  assertExists(mockContext.plugin_args);
  assertExists(mockContext.manifest);
  assertExists(mockContext.config);
  assertExists(mockContext.project_variables);
  assertExists(mockContext.project_root);
  assertExists(mockContext.meta);
  assertEquals(typeof mockContext.dry_run, "boolean");

  // Verify manifest structure
  assertExists(mockContext.manifest.plugin);
  assertExists(mockContext.manifest.commands);
  assertExists(mockContext.manifest.deno_dependencies);
  assertEquals(mockContext.manifest.plugin.name, "test-plugin");
  assertEquals(mockContext.manifest.commands.length, 2);
});

Deno.test("PluginMeta - structure validation", () => {
  const meta: PluginMeta = {
    name: "my-plugin",
    version: "2.1.0",
    description: "A useful plugin",
    registry: "https://github.com/user/repo.git",
  };

  assertEquals(meta.name, "my-plugin");
  assertEquals(meta.version, "2.1.0");
  assertEquals(meta.description, "A useful plugin");
  assertEquals(meta.registry, "https://github.com/user/repo.git");
});

Deno.test("PluginResult - success variant", () => {
  const successResult: PluginResult = {
    success: true,
    data: {
      message: "Operation completed",
      count: 42,
    },
  };

  assertEquals(successResult.success, true);
  assertExists(successResult.data);
  assertEquals(successResult.data.message, "Operation completed");
  assertEquals(successResult.data.count, 42);
});

Deno.test("PluginResult - error variant", () => {
  const errorResult: PluginResult = {
    success: false,
    error: "Something went wrong",
  };

  assertEquals(errorResult.success, false);
  assertEquals(errorResult.error, "Something went wrong");
});

Deno.test("PluginManifest - complete structure", () => {
  const manifest: PluginManifest = {
    plugin: {
      name: "deployment-tool",
      version: "1.5.0",
      description: "Deploy applications",
      registry: "https://github.com/company/plugins.git",
    },
    commands: ["deploy", "rollback", "status"],
    deno_dependencies: {
      "cliffy": "https://deno.land/x/cliffy@v1.0.0-rc.3/mod.ts",
      "zod": "https://deno.land/x/zod@v3.22.4/mod.ts",
    },
    registry: "https://github.com/company/plugins.git",
  };

  assertEquals(manifest.plugin.name, "deployment-tool");
  assertEquals(manifest.commands.length, 3);
  assertEquals(Object.keys(manifest.deno_dependencies).length, 2);
  assertEquals(manifest.registry, "https://github.com/company/plugins.git");
});
