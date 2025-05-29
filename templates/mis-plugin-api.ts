/**
 * ⚠️ AUTO-GENERATED FILE — DO NOT MODIFY ⚠️
 *
 * This file was created by the Make It So CLI and is used by plugins
 * to interface with the plugin runtime.
 *
 * Any changes to this file may break plugin functionality.
 * To update it, re-run `mis init` or upgrade your CLI version.
 */

import { PluginContext, PluginResult } from "./mis-types.d.ts";

export async function loadContext(): Promise<PluginContext> {
  const reader = Deno.stdin.readable
    .pipeThrough(new TextDecoderStream())
    .getReader();
  const { value } = await reader.read();
  return JSON.parse(value || "") as PluginContext;
}


export async function runPlugin<T = unknown>(
  command: string,
  args: Record<string, unknown> = {},
  options: { debug?: boolean } = {}
): Promise<PluginResult> {
  const proc = new Deno.Command("mis", {
    args: ["run", command, ...Object.entries(args).flatMap(([k, v]) => [`--${k}`, String(v)])],
    stdout: "piped",
    stderr: "piped",
  });

  // Check for debug mode - explicit option or MIS_DEBUG env var
  const debug = options.debug || Deno.env.get("MIS_DEBUG") === "true";

  if (debug) {
    console.error(`🔍 Running: mis run ${command} ${Object.entries(args).flatMap(([k, v]) => [`--${k}`, String(v)]).join(' ')}`);
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
      error: `Plugin '${command}' failed with exit code ${code}:\n${errorOutput}`,
    };
  }

  try {
    // Extract the final JSON result from potentially mixed output
    const result = extractFinalJson(output);
    
    if (debug) {
      console.error(`🔍 Parsed JSON: ${JSON.stringify(result)}`);
    }
    
    return result as PluginResult;
  } catch (err) {
    return {
      success: false,
      error: `Plugin '${command}' returned invalid JSON.\n\nFull output:\n${output}\n\nParse error: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}

/**
 * Runs a plugin and automatically handles errors by outputting JSON and exiting.
 * Perfect for composition plugins - no error handling boilerplate needed.
 */
export async function runPluginSafe<T = unknown>(
  command: string,
  args: Record<string, unknown> = {},
  options: { debug?: boolean } = {}
): Promise<T> {
  const result = await runPlugin<T>(command, args, options);

  if (!result.success) {
    console.log(JSON.stringify({
      success: false,
      error: `Plugin '${command}' failed: ${result.error}`
    }, null, 2));
    Deno.exit(1);
  }

  if (!result.data) {
    console.log(JSON.stringify({
      success: false,
      error: `Plugin '${command}' returned no data`
    }, null, 2));
    Deno.exit(1);
  }

  return result.data as T;
}

/**
 * Composes multiple plugins in sequence, passing data between them.
 * Super simple way to build composition plugins.
 */
export async function composePlugins<T = unknown>(
  steps: Array<{
    plugin: string;
    args?: Record<string, unknown> | ((previousResult: unknown) => Record<string, unknown>);
    transform?: (result: unknown) => unknown;
  }>,
  options: { debug?: boolean } = {}
): Promise<T> {
  let previousResult: unknown = null;
  let finalResult: unknown = null;

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    
    // Calculate args for this step
    let stepArgs: Record<string, unknown> = {};
    if (typeof step.args === 'function') {
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
export async function composePluginsWithContext(
  context: PluginContext, // Always the CLI context - predictable and explicit
  steps: Array<{
    plugin: string;
    transform?: (result: PluginResult) => PluginResult;
  }>,
  options: { 
    debug?: boolean,
    pluginResolver?: (pluginName: string, projectRoot: string) => string
  } = {}
): Promise<PluginContext> {
  // Initialize results array if not present
  let enrichedContext: PluginContext = {
    ...context,
    results: context.results || []
  };

  const resolver = options.pluginResolver || defaultPluginResolver;

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    
    if (options.debug || Deno.env.get("MIS_DEBUG") === "true") {
      console.error(`🔍 Step ${i + 1}/${steps.length}: ${step.plugin}`);
    }

    // Resolve plugin path using configurable resolver
    const pluginPath = resolver(step.plugin, context.project_root);

    // Run the plugin with the enriched context
    const proc = new Deno.Command("deno", {
      args: ["run", "--allow-run", "--allow-env", pluginPath],
      stdin: "piped",
      stdout: "piped", 
      stderr: "piped",
      cwd: context.project_root // Always use context.project_root - no ambiguity
    });
    
    const child = proc.spawn();
    
    // Send enriched context to plugin via stdin
    const writer = child.stdin.getWriter();
    await writer.write(new TextEncoder().encode(JSON.stringify(enrichedContext)));
    await writer.close();
    
    const { code, stdout, stderr } = await child.output();
    
    if (code !== 0) {
      const errorOutput = new TextDecoder().decode(stderr);
      throw new Error(`Plugin '${step.plugin}' failed: ${errorOutput}`);
    }
    
    const output = new TextDecoder().decode(stdout);
    
    if (options.debug || Deno.env.get("MIS_DEBUG") === "true") {
      console.error(`🔍 Plugin output: ${output}`);
      console.error(`🔍 Plugin stderr: ${new TextDecoder().decode(stderr)}`);
    }
    
    // Parse the plugin result using robust JSON extraction
    try {
      const result = extractFinalJson(output) as PluginResult;
      
      // Transform result if needed
      const finalResult = step.transform ? step.transform(result) : result;
      
      // Accumulate the result in the context
      enrichedContext.results!.push({
        plugin: step.plugin,
        success: finalResult.success,
        data: finalResult.success ? finalResult.data : undefined,
        error: finalResult.success ? undefined : finalResult.error,
        timestamp: new Date().toISOString()
      });
      
      // If this plugin updated the context, merge those changes
      if (finalResult.success && finalResult.context) {
        enrichedContext = {
          ...enrichedContext,
          ...finalResult.context,
          results: enrichedContext.results // Keep our accumulated results
        };
      }
      
    } catch (err) {
      throw new Error(`Plugin '${step.plugin}' returned invalid JSON: ${err instanceof Error ? err.message : String(err)}\n\nFull output:\n${output}`);
    }
  }

  return enrichedContext;
}

export function assertRequiredArgs(_args: Record<string, unknown>, _requiredArgs: string[]) {
  // TODO: Implement argument validation
}

/**
 * Default plugin path resolver - can be overridden for custom plugin layouts
 */
export function defaultPluginResolver(pluginName: string, projectRoot: string): string {
  return `${projectRoot}/.makeitso/plugins/${pluginName.replace(':', '/')}.ts`;
}

/**
 * Extract the last valid JSON object from mixed output.
 * Handles cases where plugins output debug info followed by result JSON.
 */
export function extractFinalJson(output: string): unknown {
  const lines = output.trim().split('\n');
  
  // Try to find the last complete JSON object
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    
    // Skip empty lines
    if (!line) continue;
    
    // Try parsing this line as JSON
    if (line.startsWith('{')) {
      try {
        return JSON.parse(line);
      } catch {
        // Try parsing from this line to the end (multi-line JSON)
        const remainingLines = lines.slice(i).join('\n').trim();
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
 * Makes plugin development braindead simple - no JSON boilerplate needed!
 */
export function outputSuccess(data: Record<string, unknown>, context?: PluginContext): never {
  console.log(JSON.stringify({
    success: true,
    data,
    ...(context ? { context } : {})
  }, null, 2));
  Deno.exit(0);
}

/**
 * Output an error plugin result and exit.
 * Makes error handling braindead simple - no JSON boilerplate needed!
 */
export function outputError(error: string, context?: PluginContext): never {
  console.log(JSON.stringify({
    success: false,
    error,
    ...(context ? { context } : {})
  }, null, 2));
  Deno.exit(1);
}