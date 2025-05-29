use std::fs;

use crate::utils::find_project_root;

pub fn create_plugin(name: &str) -> anyhow::Result<()> {
    let root_dir = find_project_root()
        .ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    let makeitso_dir = root_dir.join(".makeitso");

    if !makeitso_dir.exists() {
        anyhow::bail!(
            "🛑 No Make It So project found in this directory.\n→ Run `mis init` first to initialize your project."
        );
    }

    let plugin_dir = makeitso_dir.join("plugins").join(name);

    if plugin_dir.exists() {
        anyhow::bail!("Plugin '{}' already exists", name);
    }

    fs::create_dir_all(&plugin_dir)?;

    // Write scaffold files - no longer creating local types.d.ts since we use shared files
    fs::write(plugin_dir.join(format!("{}.ts", name)), scaffold_ts(name))?;
    fs::write(plugin_dir.join("plugin.toml"), scaffold_toml(name))?;

    println!("✅ Created plugin '{}'", name);

    Ok(())
}

fn scaffold_ts(name: &str) -> String {
    format!(
        r#"// Import shared types and utilities from Make It So
import type {{ PluginContext }} from "../mis-types.d.ts";
import {{ loadContext, outputSuccess, outputError }} from "../mis-plugin-api.ts";
        
// Import any external dependencies your plugin needs. Declare them in plugin.toml under [deno_dependencies].
// This one was declared automatically for you.
import * as cow from "https://deno.land/x/cowsay@1.1/mod.ts";

// 👇 This is the entrypoint of your plugin script.
// The Make It So CLI pipes JSON into stdin when it runs your plugin.
try {{
  // Load context using the shared utility
  const ctx: PluginContext = await loadContext();

  // Optional: inspect the context structure
  console.log(ctx);

  // Respect the dry run flag from the CLI
  if (ctx.dry_run) {{
    console.log("🚫 Dry run: skipping execution.");
    return;
  }}

  // Access your custom config from plugin.toml under [user_config]
  const message = String(ctx.config.message ?? "Hello from {name} 🪄");

  // Do your thing — in this case, print a talking cow 🐮
  console.log("Hello from {name}!")
  console.log(cow.say({{ text: message }}));

  // Output success result using shared utility
  outputSuccess({{ message: "Plugin executed successfully!" }});

}} catch (error) {{
  // Output error result using shared utility
  outputError(error instanceof Error ? error.message : String(error));
}}
"#,
        name = name
    )
}


fn scaffold_toml(name: &str) -> String {
    format!(
        r#"# 👇 This is your plugin manifest. It tells Make It So how to run your plugin.
[plugin]
name = "{name}"
version = "0.1.0"
description = "A plugin scaffolded by Make It So."

# 👇 These are the CLI commands this plugin supports
[commands.moo]
description = "Moo!!!!"
script = "./{name}.ts"

# 👇 These are any external dependencies your plugin needs
[deno_dependencies]
# You can import any Deno-compatible module here by name
# Example: using `cowsay` to print a message
cowsay = "https://deno.land/x/cowsay@1.1/mod.ts"

# 👇 Everything below here is YOUR plugin-specific config that you can access in .ts via the ctx variable.
[user_config]
# Message to print out — this config is passed into your plugin via ctx.config
message = "Moo It So 🪄"
"#,
    name = name
    )
}
