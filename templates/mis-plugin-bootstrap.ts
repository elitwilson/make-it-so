// Import shared types and utilities from Make It So
import type { PluginContext } from "../../mis-types.d.ts";
import { mis } from "../../mis-plugin-api.ts";

// Import any external dependencies your plugin needs. Declare them in manifest.toml under [deno_dependencies].
// This one was declared automatically for you.
import * as cow from "https://deno.land/x/cowsay@1.1/mod.ts";

// 👇 This is the entrypoint of your plugin script.
// The Make It So CLI pipes JSON into stdin when it runs your plugin.
try {
  // Load context using the shared utility
  const ctx: PluginContext = await mis.loadContext();

  // Optional: inspect the context structure
  console.log("🔍 Plugin Context:", {
    pluginName: ctx.meta.name,
    version: ctx.meta.version,
    registry: ctx.meta.registry,
    availableCommands: ctx.manifest.commands,
    configKeys: Object.keys(ctx.config),
    argsReceived: Object.keys(ctx.plugin_args),
  });

  // Respect the dry run flag from the CLI
  if (ctx.dry_run) {
    console.log("🚫 Dry run: skipping execution.");
    Deno.exit(0);
  }

  // NEW: Use helper functions to access data safely with defaults

  // Access user configuration from config.toml with default fallback
  const message = mis.getConfig(ctx, "message", "Hello from examples 🪄");
  const theme = mis.getConfig(ctx, "theme", "default");

  // Access CLI arguments passed by user
  const userMessage = mis.getArg(ctx, "message");

  // Access project-level variables
  const projectName = mis.getProjectVar(ctx, "name", "unnamed-project");

  // Use CLI argument if provided, otherwise use config, otherwise use default
  const finalMessage = userMessage || message;

  // NEW: Check plugin capabilities using helper functions
  if (mis.hasDependency(ctx, "cowsay")) {
    console.log(`✅ Using cowsay from: ${mis.getDependencyUrl(ctx, "cowsay")}`);
  }

  // Show information about the plugin's runtime context
  console.log(`🔧 Running plugin: ${ctx.meta.name} v${ctx.meta.version}`);
  console.log(`📁 Project: ${projectName} (${ctx.project_root})`);
  console.log(`🎨 Theme: ${theme}`);

  if (ctx.meta.registry) {
    console.log(`📦 Installed from: ${ctx.meta.registry}`);
  }

  // Do your thing — in this case, print a talking cow 🐮
  console.log("Hello from examples!");
  console.log(cow.say({ text: finalMessage }));

  // Output success result using shared utility
  mis.outputSuccess({
    message: "Plugin executed successfully!",
    config_used: { message, theme },
    args_received: ctx.plugin_args,
    project: projectName,
  });
} catch (error) {
  // Output error result using shared utility
  mis.outputError(error instanceof Error ? error.message : String(error));
}
