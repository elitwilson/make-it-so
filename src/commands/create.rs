use std::fs;
use std::path::PathBuf;

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

    // Write scaffold files
    fs::write(plugin_dir.join(format!("{}.ts", name)), scaffold_ts(name))?;
    fs::write(plugin_dir.join("plugin.toml"), scaffold_toml(name))?;
    fs::write(plugin_dir.join("types.d.ts"), scaffold_types())?;

    println!("✅ Created plugin '{}'", name);

    Ok(())
}

fn scaffold_ts(name: &str) -> String {
    format!(
        r#"// Import any external dependencies your plugin needs. Declare them in plugin.toml under [deno_dependencies].
// This one was declared automatically for you.
import * as cow from "https://deno.land/x/cowsay@1.1/mod.ts";

// Read plugin context from stdin (injected by Make It So CLI)
const decoder = new TextDecoder("utf-8");

// 👇 This is the entrypoint of your plugin script.
// The Make It So CLI pipes JSON into stdin when it runs your plugin.
Deno.stdin.readable
  .pipeThrough(new TextDecoderStream())
  .getReader()
  .read()
  .then(({{ value }}) => {{
    const data = value || "";

    // 👇 This is the runtime context injected by the CLI
    const ctx = JSON.parse(data) as PluginContext;

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
    }});
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
entrypoint = "moo"

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


fn scaffold_types() -> &'static str {
    r#"export type PluginContext = {
  plugin_args: Record<string, string | boolean>;
  config: Record<string, unknown>;
  project_root: string;
  env: Record<string, string>;
  meta: {
    plugin_name: string;
    plugin_description: string;
    plugin_version: string;
    cli_version: string;
  };
  dry_run: boolean;
  log: (msg: string) => void;
};
"#
}
