// Import shared types and utilities from Make It So
import type { PluginContext } from "../../mis-types.d.ts";
import { mis } from "../../mis-plugin-api.ts";
        
// Import any external dependencies your plugin needs. Declare them in plugin.toml under [deno_dependencies].
// This one was declared automatically for you.
import * as cow from "https://deno.land/x/cowsay@1.1/mod.ts";

// 👇 This is the entrypoint of your plugin script.
// The Make It So CLI pipes JSON into stdin when it runs your plugin.
try {
  // Load context using the shared utility
  const ctx: PluginContext = await mis.loadContext();

  // Optional: inspect the context structure
  console.log(ctx);

  // Respect the dry run flag from the CLI
  if (ctx.dry_run) {
    console.log("🚫 Dry run: skipping execution.");
    Deno.exit(0);
  }

  // Access your custom config from plugin.toml under [user_config]
  const message = String(ctx.config.message ?? "Hello from examples 🪄");

  // Do your thing — in this case, print a talking cow 🐮
  console.log("Hello from examples!")
  console.log(cow.say({ text: message }));

  // Output success result using shared utility
  mis.outputSuccess({ message: "Plugin executed successfully!" });

} catch (error) {
  // Output error result using shared utility
  mis.outputError(error instanceof Error ? error.message : String(error));
}
