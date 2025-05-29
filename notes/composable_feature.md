# 🧩 Plugin Composition Spec (Draft)

This document outlines a proposed mechanism for enabling **safe, automated plugin composition** in the `Make It So` CLI framework.

---

## 🎯 Motivation

To support plugin orchestration via:
- ✅ CLI wizards
- ✅ Codegen of orchestrator `.ts` files
- ✅ Workflow validation and discoverability

...we need plugins to **declare their inputs and outputs** in a standard format.

---

## 🧱 Goals

- Make plugin **I/O contract-based** and **introspectable**
- Enable the CLI to **validate and connect compatible plugins**
- Allow generation of **orchestrator plugins** from declarative recipes
- Provide guardrails to avoid invalid plugin chaining

---

## 🧩 Proposed TOML Format Extensions

### Example

```toml
[plugin]
name = "git-utils"
version = "0.1.0"

[commands.bumpSemver]
description = "Bump Git version tag"
script = "./bumpSemver.ts"

[commands.bumpSemver.outputs]
version = "string"

[commands.deploy.inputs]
version = "string"
🛠 Supported Types

Primitive types:

    string

    boolean

    integer

    float

Structured types (future):
[types.BuildArtifact]
fields.name = "string"
fields.path = "string"
fields.checksum = "string"

[commands.build.outputs]
artifact = "BuildArtifact"

[commands.upload.inputs]
artifact = "BuildArtifact"
🔄 Composition Logic

    CLI parses all plugin.toml files

    For each plugin command:

        Capture its declared inputs and outputs

    For composition (e.g. wizard or codegen):

        Match compatible input/output keys and types

        Auto-generate orchestrator .ts plugins or pipeline suggestions

💬 Example Workflow (CLI Wizard)
? Select initial plugin: git-utils:bumpSemver
✔ Produces: version (string)

? Compatible next step: [campsites:deploy]
✔ Accepts: version (string)

? Name your orchestrator plugin: release:shipIt
→ Plugin created at .makeitso/plugins/release/shipIt.ts
🧪 Runtime Considerations

    Plugins must output JSON in PluginResult format

    Outputs must include keys declared in plugin.toml

    CLI can validate contract violations (missing output, bad type, etc.)

🚫 Out of Scope (for now)

    Dynamic runtime type coercion

    Multiple output variants (union types)

    Conditional flows or plugins with side-channel state

📌 Summary

This spec introduces a forward-looking system for composable plugin metadata.
It is not required today, but it unlocks:

    Scaffolding of complex workflows

    Wizard-driven plugin orchestration

    Safer plugin re-use via declarative I/O

Let’s keep plugins composable, introspectable, and clean.

🧠 Future You will thank you.


✅ Context Accumulation - Like Go's http.Context!
This is a much more powerful and flexible architecture:
Key Benefits:
📚 Full History Access: Each plugin can see ALL previous plugin results, not just the immediate predecessor
🔍 Selective Data Access: Plugins can find specific results by plugin name:
Apply to mis-utils.ts
;
🕐 Timestamps: Each result includes when it was executed for debugging/auditing
💡 Complex Workflows: Later plugins can combine data from multiple earlier plugins
🧠 Rich Context: The context grows richer as it flows through the pipeline
Data Flow Example:
Apply to mis-utils.ts
}
Advanced Workflow Patterns Now Possible:
Multi-step validation: Later plugins can validate against multiple earlier results
Conditional logic: Plugins can branch based on what previous plugins did
Data aggregation: Final plugins can summarize all previous operations
Error recovery: Plugins can see which earlier steps failed and why
Audit trails: Complete record of what happened and when
Developer Experience:
✅ Simple: Plugins just access ctx.results to see previous work
✅ Type-safe: Full TypeScript support with proper types
✅ Discoverable: Easy to find results by plugin name
✅ Rich: Includes success/error state, data, and timestamps
This is a much more sophisticated and powerful composition system than simple linear data passing! It enables complex workflow orchestration while keeping individual plugins simple and focused. Brilliant idea! 🎯