/**
 * ⚠️ AUTO-GENERATED FILE — DO NOT MODIFY ⚠️
 *
 * This file was created by the Make It So CLI and is used by plugins
 * to interface with the plugin runtime.
 *
 * Any changes to this file may break plugin functionality.
 * To update it, re-run `mis init` or upgrade your CLI version.
 */

export type PluginContext = {
  plugin_args: Record<string, unknown>;
  manifest: PluginManifest; // Plugin metadata (from manifest.toml)
  config: Record<string, unknown>; // User configuration (from config.toml)
  project_variables: Record<string, unknown>; // Project-level variables
  project_root: string;
  meta: PluginMeta;
  dry_run: boolean;
  results?: Array<{
    plugin: string;
    success: boolean;
    data?: Record<string, unknown>;
    error?: string;
    timestamp?: string;
  }>;
};

export type PluginManifest = {
  plugin: PluginMeta;
  commands: string[]; // Available command names
  deno_dependencies: Record<string, string>;
  registry?: string; // Registry URL where plugin was installed from
};

export type PluginMeta = {
  name: string;
  description?: string;
  version: string;
  registry?: string;
};

export type PluginResult =
  | {
    success: true;
    data: Record<string, unknown>; // actual payload returned by the plugin
    context?: PluginContext; // passthrough context for composition
  }
  | {
    success: false;
    error: string; // human-readable message
    context?: PluginContext; // passthrough context even on failure
  };
