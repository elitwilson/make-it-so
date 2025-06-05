/**
 * ⚠️ AUTO-GENERATED FILE — DO NOT MODIFY ⚠️
 *
 * This file was created by the Make It So CLI and is used by plugins
 * to interface with the plugin runtime.
 *
 * Any changes to this file may break plugin functionality.
 * To update it, re-run `mis init` or upgrade your CLI version.
 */

/**
 * Plugin execution context with optional type-safe configuration.
 *
 * @template TConfig - User-provided interface matching your config.toml structure for type safety
 *
 * @example
 * // Without typing (backward compatible):
 * const ctx = await mis.loadContext();
 *
 * @example
 * // With type safety:
 * interface MyConfig { database: { host: string; }; }
 * const ctx = await mis.loadContext<MyConfig>();
 * // ctx.config.database.host is now fully typed!
 */
export type PluginContext<TConfig = Record<string, unknown>> = {
  plugin_args: Record<string, unknown>;
  manifest: PluginManifest; // Plugin metadata (from manifest.toml)
  config: TConfig; // User configuration (from config.toml)
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

export type PluginResult<TConfig = Record<string, unknown>> =
  | {
    success: true;
    data: Record<string, unknown>; // actual payload returned by the plugin
    context?: PluginContext<TConfig>; // passthrough context for composition
  }
  | {
    success: false;
    error: string; // human-readable message
    context?: PluginContext<TConfig>; // passthrough context even on failure
  };

// Helper type for common sectioned config pattern
export type SectionedConfig<T> = {
  [K in keyof T]: T[K];
};
