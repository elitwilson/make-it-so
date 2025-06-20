# Make It So

> ⚠️ **Project Status: Alpha**\
> This project is in active development. Expect breaking changes as APIs and
> features evolve.\
> Feedback and contributions are welcome!

**Make It So** is a CLI framework that lets you build your own project-specific
CLI commands using TypeScript plugins powered by Deno.

[![Version](https://img.shields.io/github/v/release/elitwilson/make-it-so)](https://github.com/elitwilson/make-it-so/releases)
[![Build](https://github.com/elitwilson/make-it-so/actions/workflows/release.yml/badge.svg)](https://github.com/elitwilson/make-it-so/actions/workflows/release.yml)
[![License](https://img.shields.io/github/license/elitwilson/make-it-so)](https://github.com/elitwilson/make-it-so/blob/main/LICENSE)

# Installation

You can install `mis` via Homebrew (macOS/Linux) or Scoop (Windows).

---

## macOS / Linux (via Homebrew 🍺)

```sh
brew install elitwilson/make-it-so/mis
```

## Windows (via Scoop)

```sh
scoop bucket add make-it-so https://github.com/elitwilson/scoop-make-it-so
scoop install mis
```

## Quickstart

```sh
mis init                # <-- Init a .makeitso directory in your desired project
mis create my-plugin    # <-- Create a new plugin via scaffolding
mis run my-plugin:moo   # <-- Run the "moo" command in the newly created "my-plugin"
```

## What It Does

- Creates a `.makeitso/` folder in your current directory with TypeScript API
  files.
- Lets you define your own CLI commands with scaffolded TypeScript plugins.
- Each plugin runs in Deno and can define its own dependencies and config.
- Provides rich TypeScript types and utilities for plugin development.
- Supports plugin composition for building complex workflows. 🚧 WIP
- Keeps everything project-local — no global installs or `node_modules` clutter.

## Security

Make It So is built with security in mind, and Deno was chosen specifically for\
its secure-by-default execution model. Plugins run in a sandboxed environment\
with explicit, validated permissions:

- File access is limited to the project directory
- Network and command execution are disabled by default
- Dangerous paths, commands, and hosts are blocked via internal validation

Active development is focused on manifest-based permission config, user prompts\
for escalations, and a possible trust model for plugin authors. Until then,\
only explicitly granted and validated permissions are allowed.

## Plugin Workflow

1. `mis init`\
   Creates `.makeitso/` with config file and TypeScript API files for plugin
   development.

2. `mis create my-plugin`\
   Scaffolds a plugin inside `.makeitso/plugins/my-plugin` with proper
   TypeScript imports.

3. `mis run my-plugin:your-command`\
   Runs a specific command defined by your plugin.

## TypeScript Development Experience

When you run `mis init`, Make It So creates TypeScript API files in your
`.makeitso/` directory:

```
.makeitso/
├── mis.toml              # Project configuration
├── plugin-types.d.ts     # TypeScript type definitions
└── plugin-api.ts         # Utilities for plugin development
```

Your plugins automatically get:

- **Full TypeScript support** with proper type definitions
- **Rich context object** with plugin args, config, and project variables
- **Utility functions** for common operations (loading context, outputting
  results)
- **Plugin composition utilities** for building complex workflows

### Plugin Template Structure

Generated plugins use the shared API:

```ts
// Import shared types and utilities from Make It So
import type { PluginContext } from "../mis-plugin-types.d.ts";
import { mis } from "../mis-plugin-api.ts";

try {
  // Load context using the shared utility
  const ctx: PluginContext = await mis.loadContext();

  // Your plugin logic here...

  // Output success result
  mis.outputSuccess({ message: "Plugin executed successfully!" });
} catch (error) {
  mis.outputError(error instanceof Error ? error.message : String(error));
}
```

## Example Command

The scaffolded plugin includes a `moo` command using the
[`cowsay`](https://deno.land/x/cowsay) library:

```sh
mis run my-plugin:moo
```

You'll see:

```
 ____________
< Make It So 🪄 >
 ------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

### First-time setup? No problem.

When you run `mis init`, Make It So checks if Deno is installed. If not, you'll
be prompted:

```
Deno is not installed. Would you like to install it? [y/N]: y
👇 Installing Deno...
######################################################################## 100.0%
Archive:  /Users/you/.deno/bin/deno.zip
  inflating: /Users/you/.deno/bin/deno  
Deno was installed successfully to /Users/you/.deno/bin/deno

Deno was added to the PATH.
You may need to restart your shell for it to become available.

Set up completions?
  [ ] bash (not recommended on macOS)
> [ ] zsh
```

Make It So will handle downloading and installing Deno for you, so you're ready
to start building plugins right away.

> ✅ You only need to do this once — future commands will just work.

## Plugin Composition (🚧 WIP)

The TypeScript API includes powerful utilities for building complex workflows by
composing multiple plugins:

```ts
import { composePlugins, runPluginSafe } from "../plugin-api.ts";

// Simple composition - pass data between plugins
const result = await composePlugins([
  {
    plugin: "validate-input",
    args: { file: "package.json" },
  },
  {
    plugin: "process-data",
    args: (prevResult) => ({ data: prevResult.processedData }),
  },
]);

// Or use individual plugin calls
const validationResult = await runPluginSafe("validate-semver", {
  version: "1.2.3",
});
```

## Plugin Structure

Each plugin lives inside `.makeitso/plugins/<your-plugin>/` and includes two
configuration files:

- **`manifest.toml`** - Static plugin metadata and commands (managed by plugin
  author)
- **`config.toml`** - User-editable configuration variables

### Plugin Manifest (`manifest.toml`)

The manifest file defines the plugin's static configuration:

```toml
[plugin]
name = "test-plugin"
version = "0.1.0"
description = "A plugin scaffolded by Make It So."

[commands.moo]
description = "Moo!!!!"
script = "./test-plugin.ts"

# ----- You can create your own commands like so: -- #
[commands.bark]               # <-- Your new command
description = "Bark!!!"       # <-- Help description for your new command
script = "./bark-plugin.ts"   # <-- create a new .ts script for every command
# -------------------------------------------------- #

[deno_dependencies]           # <-- Shared dependencies available to all plugins
cowsay = "https://deno.land/x/cowsay@1.1/mod.ts"
```

### Plugin Config (`config.toml`)

The config file contains user-customizable variables:

```toml
[user_config]                 # <-- User-customizable config variables
message = "Moo It So 🪄"      # <-- Accessible via 'ctx.config' in your .ts file
```

### Manifest Fields

#### Plugin Metadata

| Field         | Type   | Description                            |
| ------------- | ------ | -------------------------------------- |
| `name`        | string | Plugin name (should match folder name) |
| `version`     | string | Plugin version (e.g. `0.1.0`)          |
| `description` | string | Description of what this plugin does   |

#### Commands

Define commands under `[commands.<command-name>]`:

| Field         | Type   | Description                      |
| ------------- | ------ | -------------------------------- |
| `description` | string | Description shown in help output |
| `script`      | string | Path to the `.ts` script to run  |

#### Dependencies

List external Deno modules used by the plugin in the manifest file:

```toml
[deno_dependencies]
cowsay = "https://deno.land/x/cowsay@1.1/mod.ts"
```

These are available in your script:

```ts
import { say } from "cowsay";
```

### Config Fields

In `config.toml`, under `[user_config]`, you can define any config your plugin
script needs. It's available via `ctx.config` in TypeScript:

```ts
// Access plugin config
console.log("message:", ctx.config.message);

// Access plugin arguments
console.log("args:", ctx.plugin_args);

// Access project variables
console.log("project vars:", ctx.project_variables);
```

---

## Available Commands

| Command                    | Description                         | Status   |
| -------------------------- | ----------------------------------- | -------- |
| `mis init`                 | Initialize a new Make It So project | ✅ Ready |
| `mis create <plugin>`      | Create a new plugin                 | ✅ Ready |
| `mis run <plugin:command>` | Run a plugin command                | ✅ Ready |
| `mis add <plugin>`         | Install plugins from registry       | 🚧 WIP   |

## Planned Features

| Feature                       | Description                                                                                        | Status |
| ----------------------------- | -------------------------------------------------------------------------------------------------- | ------ |
| Plugin-scoped security config | Specify Deno sandboxing settings on a plugin level. (Currently set to conservative defaults)       | 🚧 WIP |
| Plugin Composition/Workflows  | A primary goal of Make It So is to allow for script-chaining via composition for complex workflows | 🚧 WIP |
| Full Documentation            | Docs for CLI usage, Typescript plugin API, etc... to come                                          | 🚧 WIP |

---
## VS Code Setup

 To ensure proper editor support without interfering with your main project’s config, VS Code should be set up as a multi-root workspace.

1. Create a workspace file
At the root of your project, create `project.code-workspace`:
```json
{
  "folders": [
    {
      "name": "Main Project",
      "path": "."
    },
    {
      "name": "Make It So Plugins",
      "path": "./.makeitso"
    }
  ],
  "extensions": {
    "recommendations": [
      "denoland.vscode-deno"
    ]
  }
}
```

2. Add `.vscode/settings.json` inside `.makeitso/`

```json
{
  "deno.enable": true,
  "deno.unstable": true,
  "deno.lint": true,
  "deno.suggest.imports.hosts": {
    "https://deno.land": true
  },
  "[typescript]": {
    "editor.defaultFormatter": "denoland.vscode-deno"
  },
  "deno.importMap": "./import_map.json"
}
```

3. Leave the root `.vscode/settings.json` Deno-free  
  This keeps Deno tooling scoped only to `.makeitso` and avoids conflicts with your main project setup.

4. Open the workspace in VS Code
Use the workspace file to launch VS Code:
`code project.code-workspace`

#### You’ll now have full Deno support inside .makeitso without affecting the rest of your project.
---

## ✨ That's It

Build your own CLI commands for your project, powered by Deno + TypeScript, all
wrapped in a slick developer workflow with full TypeScript support and plugin
composition capabilities.
