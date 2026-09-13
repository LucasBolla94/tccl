// Runs the TCCL WebAssembly engine off the main thread. The page terminates this
// worker if a request takes too long, which also discards the simulated session.
let engine = null;

async function load(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`cannot load the engine (${response.status})`);
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {});
  engine = instance.exports;
}

function request(obj) {
  const input = new TextEncoder().encode(JSON.stringify(obj));
  const ptr = engine.tccl_alloc(input.length);
  new Uint8Array(engine.memory.buffer, ptr, input.length).set(input);
  const out = engine.tccl_request(ptr, input.length);
  engine.tccl_free(ptr, input.length);
  const len = new DataView(engine.memory.buffer).getUint32(out, true);
  const text = new TextDecoder().decode(new Uint8Array(engine.memory.buffer, out + 4, len).slice());
  engine.tccl_free(out, len + 4);
  return JSON.parse(text);
}

self.onmessage = async (event) => {
  const { id, init, req } = event.data;
  try {
    if (init) {
      await load(init);
      self.postMessage({ id, res: request({ op: "version" }) });
      return;
    }
    self.postMessage({ id, res: request(req) });
  } catch (e) {
    self.postMessage({ id, res: { ok: false, fatal: String(e && e.message ? e.message : e) } });
  }
};
