import { assertEquals, assertThrows } from "@std/testing";

// Extract the JSON extraction function from the API for testing
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

Deno.test("extractFinalJson - extracts JSON from clean output", () => {
  const output = '{"success": true, "data": {"message": "Hello"}}';
  const result = extractFinalJson(output);

  assertEquals(result, {
    success: true,
    data: {
      message: "Hello",
    },
  });
});

Deno.test("extractFinalJson - extracts JSON from mixed output", () => {
  const output = `
🔍 Running plugin...
Debug: Loading config
Processing files...
{"success": true, "data": {"files_processed": 5}}
`;

  const result = extractFinalJson(output);
  assertEquals(result, {
    success: true,
    data: {
      files_processed: 5,
    },
  });
});

Deno.test("extractFinalJson - handles multi-line JSON", () => {
  const output = `
Some debug output
{
  "success": true,
  "data": {
    "message": "Multi-line result",
    "details": {
      "count": 10,
      "status": "complete"
    }
  }
}
`;

  const result = extractFinalJson(output);
  assertEquals(result, {
    success: true,
    data: {
      message: "Multi-line result",
      details: {
        count: 10,
        status: "complete",
      },
    },
  });
});

Deno.test("extractFinalJson - finds last JSON when multiple present", () => {
  const output = `
{"early": "result"}
More output
{"success": false, "error": "Initial error"}
Final processing...
{"success": true, "data": {"final": "result"}}
`;

  const result = extractFinalJson(output);
  assertEquals(result, {
    success: true,
    data: {
      final: "result",
    },
  });
});

Deno.test("extractFinalJson - throws error for no JSON", () => {
  const output = "Just some text output with no JSON";

  assertThrows(
    () => extractFinalJson(output),
    Error,
    "No valid JSON found in output",
  );
});

Deno.test("extractFinalJson - handles empty output", () => {
  const output = "";

  assertThrows(
    () => extractFinalJson(output),
    Error,
    "No valid JSON found in output",
  );
});

Deno.test("extractFinalJson - handles whitespace only", () => {
  const output = "   \n  \n  ";

  assertThrows(
    () => extractFinalJson(output),
    Error,
    "No valid JSON found in output",
  );
});

Deno.test("extractFinalJson - handles malformed JSON", () => {
  const output = `
Debug output
{broken json: "missing quotes"}
More output
`;

  assertThrows(
    () => extractFinalJson(output),
    Error,
    "No valid JSON found in output",
  );
});

Deno.test("extractFinalJson - real plugin output examples", () => {
  // Test with typical success output
  const successOutput = `
🔧 Running plugin: deployment-manager v1.0.0
📁 Project: my-app (/path/to/project)
✅ Deployment completed successfully
{"success": true, "data": {"environment": "staging", "replicas": 3}}
`;

  const successResult = extractFinalJson(successOutput);
  assertEquals(successResult, {
    success: true,
    data: {
      environment: "staging",
      replicas: 3,
    },
  });

  // Test with typical error output
  const errorOutput = `
🔧 Running plugin: deployment-manager v1.0.0
❌ Deployment failed: Connection timeout
{"success": false, "error": "Failed to connect to staging environment"}
`;

  const errorResult = extractFinalJson(errorOutput);
  assertEquals(errorResult, {
    success: false,
    error: "Failed to connect to staging environment",
  });
});
