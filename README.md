# Make It So

**Make It So** is a CLI framework that lets you build your own project-specific CLI commands using TypeScript plugins powered by Deno.

## 🚀 Quickstart

```sh
mis init                # <-- Init a .makeitso directory in your desired project
mis create my-plugin    # <-- Create a new plugin via scaffolding
mis run my-plugin:moo   # <-- Run the "moo" command in the newly created "my-plugin" 
```

### First-time setup? No problem.

When you run `mis init`, Make It So checks if Deno is installed. If not, you’ll be prompted:

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

Make It So will handle downloading and installing Deno for you, so you’re ready to start building plugins right away.

> ✅ You only need to do this once — future commands will just work.

## 🗂 What It Does

- Creates a `.makeitso/` folder in your project root.
- Lets you define your own CLI commands with scaffolded TypeScript plugins.
- Each plugin runs in Deno and can define its own dependencies and config.
- Keeps everything project-local — no global installs or `node_modules` clutter.

## 🧱 Plugin Workflow

1. `mis init`  
   Creates `.makeitso/` and a `mis.toml` config.

2. `mis create my-plugin`  
   Scaffolds a plugin inside `.makeitso/plugins/my-plugin`.

3. `mis run my-plugin:your-command`  
   Runs a specific command defined by your plugin.

## 🐄 Example Command

The scaffolded plugin includes a `moo` command using [`cowsay`](https://deno.land/x/cowsay):

```sh
mis run my-plugin:moo
```

You'll see:

```
 ____________
< Moo It So 🪄 >
 ------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## 📄 Plugin Manifest (`plugin.toml`)

Each plugin lives inside `.makeitso/plugins/<your-plugin>/` and includes a `plugin.toml` file that describes what it does and how to run it.

### 🔧 Full Example

```toml
[plugin]
name = "test-plugin"
version = "0.1.0"
description = "A plugin scaffolded by Make It So."

[commands.moo]
description = "Moo!!!!"
script = "./test-plugin.ts"
entrypoint = "moo"

[deno_dependencies]
cowsay = "https://deno.land/x/cowsay@1.1/mod.ts"

[user_config]
message = "Moo It So 🪄"
```

### 🧩 Plugin Fields

| Field          | Type   | Description                                |
|----------------|--------|--------------------------------------------|
| `name`         | string | Plugin name (should match folder name)     |
| `version`      | string | Plugin version (e.g. `0.1.0`)              |
| `description`  | string | Description of what this plugin does       |

### 🚀 Commands

Define commands under `[commands.<command-name>]`:

| Field         | Type   | Description                                |
|---------------|--------|--------------------------------------------|
| `description` | string | Description shown in help output           |
| `script`      | string | Path to the `.ts` script to run            |
| `entrypoint`  | string | The exported function to call from script  |

### 📦 Dependencies

List external Deno modules used by the plugin:

```toml
[deno_dependencies]
cowsay = "https://deno.land/x/cowsay@1.1/mod.ts"
```

These are available in your script:

```ts
import { say } from "cowsay";
```

### ⚙️ Plugin Config

Under `[user_config]`, you can define any config your plugin script needs. It's available via `ctx.config` in TypeScript:

```ts
console.log("message:", ctx.config.message);
```

---

## ✨ That's It

Build your own CLI commands for your project, powered by Deno + TypeScript, all wrapped in a slick developer workflow.

> Make it powerful. Make it flexible. **Make It So.**

