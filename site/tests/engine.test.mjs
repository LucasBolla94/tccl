// Smoke test of the WebAssembly engine as the playground uses it (run after `node site/build.mjs`).
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

const dist = path.join(path.dirname(fileURLToPath(import.meta.url)), "../dist/engine");
const file = fs.readdirSync(dist).find((f) => f.endsWith(".wasm"));
const module = new WebAssembly.Module(fs.readFileSync(path.join(dist, file)));
assert.deepEqual(WebAssembly.Module.imports(module), [], "the engine needs no imports");
const e = new WebAssembly.Instance(module, {}).exports;

function request(obj) {
  const input = Buffer.from(JSON.stringify(obj));
  const ptr = e.tccl_alloc(input.length);
  new Uint8Array(e.memory.buffer, ptr, input.length).set(input);
  const out = e.tccl_request(ptr, input.length);
  e.tccl_free(ptr, input.length);
  const len = new DataView(e.memory.buffer).getUint32(out, true);
  const text = Buffer.from(new Uint8Array(e.memory.buffer, out + 4, len)).toString();
  e.tccl_free(out, len + 4);
  return JSON.parse(text);
}

const { examples } = request({ op: "examples" });
for (const ex of examples) assert.equal(request({ op: "compile", source: ex.source }).ok, true, ex.name);
const bad = request({ op: "compile", source: "contract A\nstate total: int\nview f() -> int:\n    return totl\n", lang: "es" });
assert.equal(bad.error.code, "C006");
assert.equal(bad.error.title, "Nombre desconocido");
const coin = examples.find((x) => x.name === "cloud_coin.tccl").source;
const a = request({ op: "deploy", source: coin, args: ["1000"], from: "alice" });
assert.equal(a.ok, true);
const denied = request({ op: "call", contract: a.contract, function: "mint", args: ["@bob", "5"], from: "bob", lang: "pt" });
assert.equal(denied.ok, false);
assert.equal(denied.error.code, "R002");
for (const f of fs.readdirSync(path.join(dist, "../examples")).filter((f) => f.endsWith(".scenario"))) {
  const script = fs.readFileSync(path.join(dist, "../examples", f), "utf8");
  const report = request({ op: "scenario", script });
  assert.equal(report.failed.length, 0, `${f}: ${report.failed.join("; ")}`);
}
console.log(`engine ${file}: ${examples.length} examples compile, scenarios pass`);
