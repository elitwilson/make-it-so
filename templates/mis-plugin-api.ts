/**
 * ⚠️ AUTO-GENERATED FILE — DO NOT MODIFY ⚠️
 *
 * This file was created by the Make It So CLI and is used by plugins
 * to interface with the plugin runtime.
 *
 * Any changes to this file may break plugin functionality.
 *
 * PLUGIN CONTEXT STRUCTURE:
 * - ctx.manifest: Plugin metadata from manifest.toml (name, version, commands, dependencies, registry)
 * - ctx.config: User-editable configuration from config.toml (your custom settings)
 * - ctx.plugin_args: CLI arguments passed by the user (--arg=value)
 * - ctx.project_variables: Project-level variables from mis.toml
 * - ctx.meta: Quick access to plugin metadata (same as ctx.manifest.plugin)
 * - ctx.project_root: Absolute path to the project root
 * - ctx.dry_run: Whether this is a dry-run execution
 */

import type { PluginContext, PluginResult } from "./mis-types.d.ts";

async function loadContext<TConfig = Record<string, unknown>>(): Promise<
  PluginContext<TConfig>
> {
  // First try temp file (CLI execution mode)
  const contextFileIndex = Deno.args.findIndex((arg) =>
    arg === "--context-file"
  );

  if (contextFileIndex !== -1 && Deno.args[contextFileIndex + 1]) {
    const contextFilePath = Deno.args[contextFileIndex + 1];
    try {
      const contextData = await Deno.readTextFile(contextFilePath);
      return JSON.parse(contextData) as PluginContext<TConfig>;
    } catch (error) {
      throw new Error(
        `Failed to read context file '${contextFilePath}': ${error}`,
      );
    }
  }

  // Fallback to stdin (composition mode)
  try {
    const stdinContent = await readStdinContent();
    if (stdinContent.trim()) {
      return JSON.parse(stdinContent) as PluginContext<TConfig>;
    }
  } catch (error) {
    // stdin parsing failed, continue to error
  }

  throw new Error(
    "🛑 No context provided. Expected either --context-file <path> argument or context via stdin.\n" +
      "→ This indicates a bug in the Make It So CLI or plugin composition. Please report this issue.",
  );
}

async function readStdinContent(): Promise<string> {
  const chunks: Uint8Array[] = [];
  for await (const chunk of Deno.stdin.readable) {
    chunks.push(chunk);
  }

  // Calculate total length and concatenate chunks
  const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const combined = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    combined.set(chunk, offset);
    offset += chunk.length;
  }

  return new TextDecoder().decode(combined);
}

/**
 * Helper: Get a value from user config.toml with optional default
 *
 * @example
 * const apiKey = getConfig(ctx, "api_key", "default-key");
 * const timeout = getConfig(ctx, "timeout", 30);
 */
function getConfig<TConfig, T = unknown>(
  ctx: PluginContext<TConfig>,
  key: keyof TConfig,
  defaultValue?: T,
): T {
  return (ctx.config[key] as T) ?? (defaultValue as T);
}

/**
 * Helper: Get a CLI argument with optional default
 *
 * @example
 * const environment = getArg(ctx, "environment", "staging");
 * const force = getArg(ctx, "force", false);
 */
function getArg<TConfig, T = unknown>(
  ctx: PluginContext<TConfig>,
  key: string,
  defaultValue?: T,
): T {
  return (ctx.plugin_args[key] as T) ?? (defaultValue as T);
}

/**
 * Helper: Get a project variable with optional default
 *
 * @example
 * const projectName = getProjectVar(ctx, "name", "unnamed-project");
 */
function getProjectVar<TConfig, T = unknown>(
  ctx: PluginContext<TConfig>,
  key: string,
  defaultValue?: T,
): T {
  return (ctx.project_variables[key] as T) ?? (defaultValue as T);
}

/**
 * Helper: Check if this plugin has a specific Deno dependency
 *
 * @example
 * if (hasDependency(ctx, "oak")) {
 *   // Use oak framework
 * }
 */
function hasDependency<TConfig>(
  ctx: PluginContext<TConfig>,
  dependencyName: string,
): boolean {
  return dependencyName in ctx.manifest.deno_dependencies;
}

/**
 * Helper: Get the URL for a Deno dependency
 *
 * @example
 * const oakUrl = getDependencyUrl(ctx, "oak");
 * if (oakUrl) {
 *   console.log(`Using Oak from: ${oakUrl}`);
 * }
 */
function getDependencyUrl<TConfig>(
  ctx: PluginContext<TConfig>,
  dependencyName: string,
): string | undefined {
  return ctx.manifest.deno_dependencies[dependencyName];
}

async function runPlugin<T = unknown, TConfig = Record<string, unknown>>(
  command: string,
  args: Record<string, unknown> = {},
  options: { debug?: boolean } = {},
): Promise<PluginResult<TConfig>> {
  const proc = new Deno.Command("mis", {
    args: [
      "run",
      command,
      ...Object.entries(args).flatMap(([k, v]) => [`--${k}`, String(v)]),
    ],
    stdout: "piped",
    stderr: "piped",
  });

  // Check for debug mode - explicit option or MIS_DEBUG env var
  const debug = options.debug || Deno.env.get("MIS_DEBUG") === "true";

  if (debug) {
    console.error(
      `🔍 Running: mis run ${command} ${
        Object.entries(args).flatMap(([k, v]) => [`--${k}`, String(v)]).join(
          " ",
        )
      }`,
    );
  }

  const { code, stdout, stderr } = await proc.output();
  const output = new TextDecoder().decode(stdout);
  const errorOutput = new TextDecoder().decode(stderr);

  if (debug) {
    console.error(`🔍 Exit code: ${code}`);
    console.error(`🔍 Stdout:\n${output}`);
    console.error(`🔍 Stderr:\n${errorOutput}`);
  }

  if (code !== 0) {
    return {
      success: false,
      error:
        `Plugin '${command}' failed with exit code ${code}:\n${errorOutput}`,
    };
  }

  try {
    // Extract the final JSON result from potentially mixed output
    const result = extractFinalJson(output);

    if (debug) {
      console.error(`🔍 Parsed JSON: ${JSON.stringify(result)}`);
    }

    return result as PluginResult<TConfig>;
  } catch (err) {
    return {
      success: false,
      error:
        `Plugin '${command}' returned invalid JSON.\n\nFull output:\n${output}\n\nParse error: ${
          err instanceof Error ? err.message : String(err)
        }`,
    };
  }
}

/**
 * Runs a plugin and automatically handles errors by outputting JSON and exiting.
 * Perfect for composition plugins - no error handling boilerplate needed.
 */
async function runPluginSafe<T = unknown>(
  command: string,
  args: Record<string, unknown> = {},
  options: { debug?: boolean } = {},
): Promise<T> {
  const result = await runPlugin<T>(command, args, options);

  if (!result.success) {
    console.log(JSON.stringify(
      {
        success: false,
        error: `Plugin '${command}' failed: ${result.error}`,
      },
      null,
      2,
    ));
    Deno.exit(1);
  }

  if (!result.data) {
    console.log(JSON.stringify(
      {
        success: false,
        error: `Plugin '${command}' returned no data`,
      },
      null,
      2,
    ));
    Deno.exit(1);
  }

  return result.data as T;
}

/**
 * Composes multiple plugins in sequence, passing data between them.
 * Super simple way to build composition plugins.
 */
async function composePlugins<T = unknown>(
  steps: Array<{
    plugin: string;
    args?:
      | Record<string, unknown>
      | ((previousResult: unknown) => Record<string, unknown>);
    transform?: (result: unknown) => unknown;
  }>,
  options: { debug?: boolean } = {},
): Promise<T> {
  let previousResult: unknown = null;
  let finalResult: unknown = null;

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];

    // Calculate args for this step
    let stepArgs: Record<string, unknown> = {};
    if (typeof step.args === "function") {
      stepArgs = step.args(previousResult);
    } else if (step.args) {
      stepArgs = step.args;
    }

    if (options.debug || Deno.env.get("MIS_DEBUG") === "true") {
      console.error(`🔍 Step ${i + 1}/${steps.length}: ${step.plugin}`);
    }

    // Run the plugin
    const result = await runPluginSafe(step.plugin, stepArgs, options);

    // Transform result if needed
    finalResult = step.transform ? step.transform(result) : result;
    previousResult = finalResult;
  }

  return finalResult as T;
}

/**
 * Composes multiple plugins in sequence using context accumulation.
 * Each plugin receives the enriched context with all previous results.
 * Perfect for complex workflows where later plugins need earlier results.
 */
async function composePluginsWithContext(
  context: PluginContext, // Always the CLI context - predictable and explicit
  steps: Array<{
    plugin: string;
    transform?: (result: PluginResult) => PluginResult;
  }>,
  options: {
    debug?: boolean;
    pluginResolver?: (pluginName: string, projectRoot: string) => string;
  } = {},
): Promise<PluginContext> {
  // Initialize results array if not present
  let enrichedContext: PluginContext = {
    ...context,
    results: context.results || [],
  };

  const resolver = options.pluginResolver || defaultPluginResolver;

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];

    if (options.debug || Deno.env.get("MIS_DEBUG") === "true") {
      console.error(`🔍 Step ${i + 1}/${steps.length}: ${step.plugin}`);
    }

    // Extract plugin arguments from enriched context for runPlugin
    const pluginArgs = enrichedContext.plugin_args || {};

    // Use runPlugin to delegate to Rust CLI (proper permission handling)
    const result = await runPlugin(
      step.plugin,
      pluginArgs,
      { debug: options.debug },
    );

    // Transform result if needed
    const finalResult = step.transform ? step.transform(result) : result;

    // Accumulate the result in the context
    enrichedContext.results!.push({
      plugin: step.plugin,
      success: finalResult.success,
      data: finalResult.success ? finalResult.data : undefined,
      error: finalResult.success ? undefined : finalResult.error,
      timestamp: new Date().toISOString(),
    });

    // If this plugin updated the context, merge those changes
    if (finalResult.success && finalResult.context) {
      enrichedContext = {
        ...enrichedContext,
        ...finalResult.context,
        results: enrichedContext.results, // Keep our accumulated results
      };
    }
  }

  return enrichedContext;
}

function assertRequiredArgs(
  _args: Record<string, unknown>,
  _requiredArgs: string[],
) {
  // TODO: Implement argument validation
}

/**
 * Default plugin path resolver - can be overridden for custom plugin layouts
 */
function defaultPluginResolver(
  pluginName: string,
  projectRoot: string,
): string {
  return `${projectRoot}/.makeitso/plugins/${pluginName.replace(":", "/")}.ts`;
}

/**
 * Extract the last valid JSON object from mixed output.
 * Handles cases where plugins output debug info followed by result JSON.
 */
function extractFinalJson(output: string): unknown {
  const lines = output.trim().split("\n");

  // Try to find the last complete JSON object
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();

    // Skip empty lines
    if (!line) continue;

    // Try parsing this line as JSON
    if (line.startsWith("{")) {
      try {
        return JSON.parse(line);
      } catch {
        // Try parsing from this line to the end (multi-line JSON)
        const remainingLines = lines.slice(i).join("\n").trim();
        try {
          return JSON.parse(remainingLines);
        } catch {
          continue;
        }
      }
    }
  }

  // Fallback: try parsing the entire output
  try {
    return JSON.parse(output.trim());
  } catch {
    throw new Error("No valid JSON found in output");
  }
}

/**
 * Output a successful plugin result and exit.
 */
function outputSuccess<
  TData = Record<string, unknown>,
  TConfig = Record<string, unknown>,
>(
  data: TData,
  context?: PluginContext<TConfig>,
): never {
  console.log(JSON.stringify(
    {
      success: true,
      data,
      ...(context ? { context } : {}),
    },
    null,
    2,
  ));
  Deno.exit(0);
}

/**
 * Output an error plugin result and exit.
 */
function outputError<TConfig = Record<string, unknown>>(
  error: string,
  context?: PluginContext<TConfig>,
): never {
  console.log(JSON.stringify(
    {
      success: false,
      error,
      ...(context ? { context } : {}),
    },
    null,
    2,
  ));
  Deno.exit(1);
}

// export a mis object with all of the above api functions
export const mis = {
  loadContext,
  runPlugin,
  runPluginSafe,
  composePlugins,
  composePluginsWithContext,
  assertRequiredArgs,
  defaultPluginResolver,
  extractFinalJson,
  outputSuccess,
  outputError,
  getConfig,
  getArg,
  getProjectVar,
  hasDependency,
  getDependencyUrl,
};
