#!/usr/bin/env node

process.stdin.setEncoding("utf8");
process.stdin.on("data", (data) => {
const ctx = JSON.parse(data);

const { service_name, env_name, version, dry_run } = ctx;

console.log(`🚀 Deploying ${service_name} to ${env_name} (version: ${version})`);

if (dry_run) {
  console.log("🚫 Dry run: skipping actual deploy.");
  return;
}

// TODO: Replace this with your real deploy logic (Azure CLI, etc)
});
