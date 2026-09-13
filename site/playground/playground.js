// TCCL Playground: editor, simulated chain and panels. The engine (the real TCCL
// compiler and VM compiled to WebAssembly) runs in a Web Worker.
import { highlight } from "./highlight.mjs";

const script = document.getElementById("playground-script");
const S = JSON.parse(script.dataset.strings);
const LANG = script.dataset.lang === "pt-BR" ? "pt" : script.dataset.lang;
const WASM = script.dataset.wasm;
const TIMEOUT_MS = 5000;
const $ = (id) => document.getElementById(id);
const el = (tag, props = {}, ...children) => {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else if (v !== undefined && v !== null && v !== false) node.setAttribute(k, v === true ? "" : v);
  }
  for (const c of children.flat()) if (c !== null && c !== undefined) node.append(c instanceof Node ? c : document.createTextNode(String(c)));
  return node;
};
const storage = {
  get(k) {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  },
  set(k, v) {
    try {
      localStorage.setItem(k, v);
    } catch {
      /* private mode */
    }
  },
};

// ---------------- Engine worker ----------------
let worker;
let nextId = 1;
const pending = new Map();
let ready;

function startWorker() {
  worker = new Worker(new URL("./worker.js", import.meta.url));
  worker.onmessage = (e) => {
    const p = pending.get(e.data.id);
    if (!p) return;
    clearTimeout(p.timer);
    pending.delete(e.data.id);
    p.resolve(e.data.res);
  };
  ready = send({ init: new URL(WASM, location.href).href }, true);
  return ready;
}

function send(message, raw = false) {
  return new Promise((resolve) => {
    const id = nextId++;
    const timer = setTimeout(() => {
      pending.delete(id);
      worker.terminate();
      for (const p of pending.values()) clearTimeout(p.timer);
      pending.clear();
      notice(S.timeout, true);
      contracts = [];
      renderContracts();
      startWorker().then(() => setStatus(S.restarted));
      resolve({ ok: false, fatal: S.timeout });
    }, TIMEOUT_MS);
    pending.set(id, { resolve, timer });
    worker.postMessage(raw ? { id, ...message } : { id, req: { lang: LANG, ...message } });
  });
}

async function engine(req) {
  await ready;
  return send(req);
}

function setStatus(text, cls = "ok") {
  const s = $("engine-status");
  s.textContent = text;
  s.className = `badge ${cls}`;
}

function notice(text, bad = false) {
  const d = $("diagnostic");
  d.replaceChildren(el("div", { class: `diag ${bad ? "bad" : "ok"}`, role: "status" }, text));
}

// ---------------- Editor ----------------
const src = $("source");
const hl = $("hl").firstElementChild;
const gutter = $("gutter");
let errorLine = 0;

function renderEditor() {
  const code = src.value;
  const lines = code.split("\n");
  let html = highlight(code);
  if (errorLine > 0 && errorLine <= lines.length) {
    const parts = html.split("\n");
    parts[errorLine - 1] = `<span class="error-line">${parts[errorLine - 1] || " "}</span>`;
    html = parts.join("\n");
  }
  hl.innerHTML = html + "\n";
  gutter.replaceChildren(...lines.map((_, i) => el("div", { class: i + 1 === errorLine ? "err" : "" }, String(i + 1))));
  syncScroll();
}

function syncScroll() {
  hl.parentElement.scrollTop = src.scrollTop;
  hl.parentElement.scrollLeft = src.scrollLeft;
  gutter.scrollTop = src.scrollTop;
}

function cursor() {
  const before = src.value.slice(0, src.selectionStart);
  const line = before.split("\n").length;
  const col = before.length - before.lastIndexOf("\n");
  $("cursor").textContent = `${S.line} ${line}, ${S.column} ${col}`;
}

function insert(text) {
  const { selectionStart: a, selectionEnd: b } = src;
  src.setRangeText(text, a, b, "end");
  onInput();
}

src.addEventListener("keydown", (e) => {
  if (e.key === "Tab") {
    e.preventDefault();
    const { selectionStart: a, value } = src;
    const lineStart = value.lastIndexOf("\n", a - 1) + 1;
    if (e.shiftKey) {
      const spaces = value.slice(lineStart, lineStart + 4).match(/^ */)[0].length;
      src.setRangeText("", lineStart, lineStart + spaces, "end");
      onInput();
    } else insert("    ");
  } else if (e.key === "Enter" && !e.isComposing) {
    e.preventDefault();
    const { selectionStart: a, value } = src;
    const line = value.slice(value.lastIndexOf("\n", a - 1) + 1, a);
    let indent = line.match(/^ */)[0];
    if (/:\s*(#.*)?$/.test(line)) indent += "    ";
    insert("\n" + indent);
  }
});
src.addEventListener("paste", (e) => {
  const text = e.clipboardData?.getData("text/plain");
  if (text && text.includes("\t")) {
    e.preventDefault();
    insert(text.replace(/\t/g, "    "));
  }
});
src.addEventListener("scroll", syncScroll);
src.addEventListener("keyup", cursor);
src.addEventListener("click", cursor);
src.addEventListener("input", onInput);
src.addEventListener("dragover", (e) => e.preventDefault());
src.addEventListener("drop", (e) => {
  const f = e.dataTransfer?.files?.[0];
  if (f) {
    e.preventDefault();
    openFile(f);
  }
});

let compileTimer;
function onInput() {
  renderEditor();
  cursor();
  storage.set("tccl.playground.source", src.value);
  clearTimeout(compileTimer);
  $("compile-status").textContent = S.compiling;
  compileTimer = setTimeout(check, 350);
}

let lastInterface = null;
async function check() {
  const res = await engine({ op: "compile", source: src.value });
  if (res.fatal) return notice(res.fatal, true);
  if (res.ok) {
    errorLine = 0;
    lastInterface = res.interface;
    $("compile-status").textContent = `✔ ${S.compiles} · ${res.interface.name} · ${res.interface.bytes} bytes`;
    $("diagnostic").replaceChildren();
    renderDeploy(res.interface);
  } else {
    const e = res.error;
    errorLine = e.line;
    lastInterface = null;
    $("compile-status").textContent = `✘ ${S.compileError} ${e.line}:${e.col}`;
    renderDiagnostic(e);
    renderDeploy(null);
  }
  renderEditor();
}

function renderDiagnostic(e) {
  const actions = el("div", { class: "fix" }, el("button", { class: "btn btn-small", type: "button", onclick: () => goTo(e.line, e.col) }, `${S.goToLine} ${e.line}`));
  const suggestion = (e.help || "").match(/did you mean '([^']+)'\?/);
  if (suggestion) {
    actions.append(el("button", { class: "btn btn-small btn-primary", type: "button", onclick: () => applyRename(e.line, e.col, suggestion[1]) }, `${S.applyFix}: ${suggestion[1]}`));
  }
  if (/tabs are not allowed/.test(e.message)) {
    actions.append(el("button", { class: "btn btn-small btn-primary", type: "button", onclick: () => ((src.value = src.value.replace(/\t/g, "    ")), onInput()) }, S.applyFix));
  }
  $("diagnostic").replaceChildren(
    el(
      "div",
      { class: "diag bad" },
      el("h3", {}, `${e.code} · ${e.title} — ${e.line}:${e.col}`),
      el("p", {}, el("code", {}, e.message)),
      e.source_line ? el("pre", { class: "mono", style: "margin:6px 0;white-space:pre-wrap" }, `${e.source_line}\n${" ".repeat(Math.max(0, e.col - 1))}^`) : null,
      el("p", {}, el("strong", {}, `${S.why}: `), e.explanation),
      el("p", {}, el("strong", {}, `${S.fixLabel}: `), e.fix),
      actions,
    ),
  );
}

function offsetOf(line, col) {
  const lines = src.value.split("\n");
  let off = 0;
  for (let i = 0; i < line - 1 && i < lines.length; i++) off += lines[i].length + 1;
  return off + Math.max(0, col - 1);
}

function goTo(line, col) {
  const off = offsetOf(line, col);
  src.focus();
  src.setSelectionRange(off, off);
  const lineHeight = parseFloat(getComputedStyle(src).lineHeight) || 22;
  src.scrollTop = Math.max(0, (line - 5) * lineHeight);
  cursor();
  syncScroll();
}

function applyRename(line, col, replacement) {
  const start = offsetOf(line, col);
  const rest = src.value.slice(start);
  const word = rest.match(/^[A-Za-z_][A-Za-z0-9_]*/);
  if (!word) return goTo(line, col);
  src.setRangeText(replacement, start, start + word[0].length, "end");
  onInput();
}

// ---------------- Files ----------------
function download(name, text, type = "text/plain") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = el("a", { href: url, download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function openFile(file) {
  if (file.size > 48000) return notice("48 000 bytes max", true);
  const reader = new FileReader();
  reader.onload = () => {
    src.value = String(reader.result).replace(/\t/g, "    ");
    onInput();
  };
  reader.readAsText(file);
}

$("open").addEventListener("click", () => $("file").click());
$("file").addEventListener("change", (e) => e.target.files[0] && openFile(e.target.files[0]));
$("save").addEventListener("click", () => {
  const name = (src.value.match(/^\s*contract\s+(\w+)/m) || [null, "contract"])[1].toLowerCase();
  download(`${name}.tccl`, src.value);
});

// ---------------- Accounts, contracts, forms ----------------
const ACCOUNTS = ["alice", "bob", "carol", "dave", "erin"];
let contracts = []; // {hex, name, address, interface}
let snapshotCache = null;

function accountSelect(select, extra) {
  select.replaceChildren(...(extra ? [el("option", { value: "none" }, extra)] : []), ...ACCOUNTS.map((a) => el("option", { value: a }, a)));
}
accountSelect($("deploy-from"));
accountSelect($("upgrade-from"));
accountSelect($("authority-to"), S.renounce);

function placeholderFor(type) {
  if (type === "int") return "42 or 1.5tcn";
  if (type === "bool") return "true";
  if (type === "text") return '"hello"';
  if (type === "bytes") return "0x…";
  if (type === "address") return "@bob or $contract";
  if (type.startsWith("list")) return "[a, b]";
  return type;
}

function paramInputs(params, container) {
  container.replaceChildren(...params.map((p) => el("label", {}, `${p.name}: ${p.type}`, el("input", { "data-param": p.name, placeholder: placeholderFor(p.type) }))));
}

function readParams(container) {
  return [...container.querySelectorAll("input[data-param]")].map((i) => i.value.trim());
}

function renderDeploy(iface) {
  $("deploy-name").textContent = iface ? `${iface.name} · language ${iface.language}` : "";
  const init = iface?.functions.find((f) => f.kind === "init");
  paramInputs(init ? init.params : [], $("deploy-params"));
  $("deploy").disabled = !iface;
}

function currentContract() {
  return contracts.find((c) => c.hex === $("contract-select").value);
}

function renderContracts() {
  const sel = $("contract-select");
  const keep = sel.value;
  sel.replaceChildren(
    ...(contracts.length ? [] : [el("option", { value: "" }, S.noContract)]),
    ...contracts.map((c) => el("option", { value: c.hex }, `${c.name} · $${c.hex.slice(0, 8)}`)),
  );
  if (contracts.some((c) => c.hex === keep)) sel.value = keep;
  renderFunctions();
  renderState();
}
$("contract-select").addEventListener("change", () => {
  renderFunctions();
  renderState();
});

function renderFunctions() {
  const c = currentContract();
  const box = $("functions");
  if (!c) return box.replaceChildren(el("p", { class: "muted" }, S.functionsEmpty));
  box.replaceChildren(
    ...c.interface.functions
      .filter((f) => f.kind === "action" || f.kind === "view")
      .map((f) => {
        const params = el("div", { class: "params" });
        paramInputs(f.params, params);
        const from = el("select", {});
        accountSelect(from);
        const value = el("input", { placeholder: "0", size: 8 });
        const run = el(
          "button",
          {
            class: `btn btn-small ${f.kind === "action" ? "btn-primary" : ""}`,
            type: "button",
            onclick: () =>
              transact(
                f.kind === "action"
                  ? { op: "call", contract: c.hex, function: f.name, args: readParams(params), from: from.value, value: value.value }
                  : { op: "view", contract: c.hex, function: f.name, args: readParams(params) },
                `${f.kind === "action" ? from.value + " → " : ""}${c.name}.${f.name}(${readParams(params).join(", ")})`,
              ),
          },
          f.kind === "action" ? S.call : S.query,
        );
        const sig = `${f.name}(${f.params.map((p) => `${p.name}: ${p.type}`).join(", ")})${f.returns !== "nothing" ? " -> " + f.returns : ""}`;
        return el(
          "div",
          { class: "fn" },
          el(
            "header",
            {},
            el("span", { class: "badge" }, f.kind),
            el("span", {}, sig),
            f.payable ? el("span", { class: "badge ok" }, "payable") : null,
            f.only ? el("span", { class: "badge bad" }, `only ${f.only.join(", ")}`) : null,
          ),
          f.params.length ? params : null,
          el("div", { class: "row" }, f.kind === "action" ? [el("label", {}, S.from, from), f.payable ? el("label", {}, S.value, value) : null] : null, run),
        );
      }),
  );
}

$("deploy").addEventListener("click", async () => {
  const res = await transact(
    { op: "deploy", source: src.value, args: readParams($("deploy-params")), from: $("deploy-from").value, value: $("deploy-value").value, final: $("deploy-final").checked },
    `${$("deploy-from").value} → deploy ${lastInterface?.name ?? ""}`,
  );
  if (res?.ok && res.contract) {
    contracts.push({ hex: res.contract, name: res.interface.name, address: res.address, interface: res.interface });
    renderContracts();
    $("contract-select").value = res.contract;
    renderFunctions();
    renderState();
  }
});

$("upgrade").addEventListener("click", async () => {
  const c = currentContract();
  if (!c) return notice(S.deployFirst, true);
  const res = await transact({ op: "upgrade", contract: c.hex, source: src.value, from: $("upgrade-from").value }, `${$("upgrade-from").value} → upgrade ${c.name}`);
  if (res?.ok && res.interface) {
    c.interface = res.interface;
    c.name = res.interface.name;
    renderContracts();
  }
});

$("authority").addEventListener("click", async () => {
  const c = currentContract();
  if (!c) return notice(S.deployFirst, true);
  const res = await engine({ op: "authority", contract: c.hex, from: $("upgrade-from").value, to: $("authority-to").value === "none" ? "none" : "@" + $("authority-to").value });
  logEntry(`${$("upgrade-from").value} → authority ${c.name} = ${$("authority-to").value}`, res.fatal ? { ok: false, error: { message: res.fatal } } : { ok: true, after: res });
  if (!res.fatal) {
    snapshotCache = res;
    renderState();
  }
});

// ---------------- Transactions and activity ----------------
async function transact(req, label) {
  const res = await engine(req);
  if (res.fatal) {
    const entry = logEntry(label, { ok: false, error: { message: res.fatal } });
    $("last-result").replaceChildren(entry.cloneNode(true));
    $("last-result").hidden = false;
    return res;
  }
  if (res.compile_error) {
    errorLine = res.compile_error.line;
    renderDiagnostic(res.compile_error);
    renderEditor();
    return res;
  }
  const entry = logEntry(label, res);
  if (res.after) snapshotCache = res.after;
  renderState(res.before);
  const last = $("last-result");
  last.replaceChildren(entry.cloneNode(true));
  last.hidden = false;
  return res;
}

function formatMotes(m) {
  const n = BigInt(m);
  const whole = n / 100000000n;
  const frac = (n % 100000000n).toString().padStart(8, "0").replace(/0+$/, "");
  return `${whole}${frac ? "." + frac : ""} TCN`;
}

function diffStates(before, after) {
  const changes = [];
  if (!before || !after) return changes;
  for (const [hex, c] of Object.entries(after.contracts)) {
    const old = before.contracts[hex];
    for (const entry of c.state) {
      const prev = old?.state.find((s) => s.name === entry.name);
      const now = entry.value + JSON.stringify(entry.items);
      const was = prev ? prev.value + JSON.stringify(prev.items) : null;
      if (now !== was) changes.push(`${c.name}.${entry.name}: ${prev ? prev.value : "—"} → ${entry.value}`);
    }
    if (old && old.balance !== c.balance) changes.push(`${c.name} ${S.balance}: ${formatMotes(old.balance)} → ${formatMotes(c.balance)}`);
  }
  for (const acc of after.accounts) {
    const old = before.accounts.find((a) => a.name === acc.name);
    if (old && old.balance !== acc.balance) changes.push(`${acc.name} ${S.balance}: ${formatMotes(old.balance)} → ${formatMotes(acc.balance)}`);
  }
  return changes;
}

function logEntry(label, res) {
  const lines = [];
  if (res.result !== undefined && res.result !== null) lines.push(el("p", {}, el("strong", {}, `${S.result}: `), el("code", {}, res.result)));
  if (res.error) {
    lines.push(el("p", {}, el("code", {}, res.error.message)));
    if (res.error.explanation) lines.push(el("p", { class: "muted" }, `${res.error.code} · ${res.error.title}: ${res.error.explanation} ${S.fixLabel}: ${res.error.fix}`));
  }
  if (res.events?.length) {
    lines.push(el("p", {}, el("strong", {}, `${S.events}:`)));
    lines.push(el("pre", {}, res.events.map((e) => `${e.contract_name ?? ""}.${e.name}(${e.fields.map(([k, v]) => `${k}: ${v}`).join(", ")})`).join("\n")));
  }
  if (res.fuel_used !== undefined) {
    const fee = res.fee && res.fee.fee ? ` · ${S.fee}: ${formatMotes(res.fee.fee)} (${res.fee.max_fuel} ${S.maxFuel}, ~${res.fee.tx_bytes} bytes) · ${S.deposit}: ${res.fee.deposit_change >= 0 ? "+" : "−"}${formatMotes(Math.abs(res.fee.deposit_change))}` : "";
    lines.push(el("p", { class: "muted" }, `${S.fuel}: ${res.fuel_used}${fee}`));
  }
  const changes = res.ok ? diffStates(res.before, res.after) : [];
  if (res.before) lines.push(el("details", {}, el("summary", {}, `${S.changes} (${changes.length})`), el("pre", {}, changes.length ? changes.join("\n") : S.noChanges)));
  if (res.report) lines.push(el("pre", {}, JSON.stringify(res.report, null, 2)));
  const status = el("span", { class: `badge ${res.ok ? "ok" : "bad"}` }, res.ok ? S.success : S.failed);
  const entry = el("div", { class: "log-entry" }, el("div", {}, status, " ", el("span", { class: "mono" }, label)), ...lines);
  $("activity").prepend(entry);
  return entry;
}

async function renderState(before) {
  const view = $("state-view");
  const snap = snapshotCache || (await engine({ op: "snapshot" }));
  snapshotCache = snap;
  $("height").value = snap.height ?? "";
  const c = currentContract();
  const parts = [];
  if (c && snap.contracts?.[c.hex]) {
    const info = snap.contracts[c.hex];
    const old = before?.contracts?.[c.hex];
    parts.push(el("h3", { style: "font-size:16px" }, `${info.name} · ${info.address}`));
    const kv = el("dl", { class: "kv" });
    const add = (k, v, changed) => kv.append(el("dt", {}, k), el("dd", { class: changed ? "changed" : "" }, v));
    add(S.balance, formatMotes(info.balance), old && old.balance !== info.balance);
    add(S.deposit, formatMotes(info.deposit));
    add(S.codeVersion, String(info.code_version), old && old.code_version !== info.code_version);
    add(S.authority, info.upgrade_authority ?? "— (final)");
    for (const entry of info.state) {
      const prev = old?.state.find((s) => s.name === entry.name);
      const changed = prev && (prev.value !== entry.value || JSON.stringify(prev.items) !== JSON.stringify(entry.items));
      const shown = entry.items.length && entry.ty !== "role" ? `${entry.value}\n${entry.items.slice(0, 50).map(([k, v]) => `  ${k} → ${v}`).join("\n")}` : entry.value || "—";
      add(`${entry.name}: ${entry.ty}`, el("span", { style: "white-space:pre-wrap" }, changed ? `${prev.value} → ${shown}` : shown), changed);
    }
    parts.push(kv);
  } else parts.push(el("p", { class: "muted" }, S.stateEmpty));
  const acc = el("dl", { class: "kv" });
  for (const a of snap.accounts || []) {
    const old = before?.accounts?.find((x) => x.name === a.name);
    acc.append(el("dt", {}, a.name), el("dd", { class: old && old.balance !== a.balance ? "changed" : "", title: a.address }, formatMotes(a.balance)));
  }
  parts.push(el("h3", { style: "font-size:16px;margin-top:18px" }, `${S.accounts} · ${S.height} ${snap.height}`), acc);
  view.replaceChildren(...parts);
}

// ---------------- Tabs ----------------
function selectTab(name) {
  for (const b of document.querySelectorAll('[role="tab"]')) {
    const on = b.id === `tab-${name}`;
    b.setAttribute("aria-selected", String(on));
    $(b.getAttribute("aria-controls")).hidden = !on;
  }
}
for (const b of document.querySelectorAll('[role="tab"]')) b.addEventListener("click", () => selectTab(b.id.replace("tab-", "")));

// ---------------- Scenario and session ----------------
$("run-scenario").addEventListener("click", async () => {
  const files = { "contract.tccl": src.value };
  const name = $("example").value;
  if (name) files[name] = src.value;
  const res = await engine({ op: "scenario", script: $("scenario").value, files });
  const box = $("scenario-report");
  if (res.fatal) return box.replaceChildren(el("div", { class: "diag bad" }, res.fatal));
  box.replaceChildren(
    el("div", { class: `diag ${res.failed.length ? "bad" : "ok"}` }, `${res.passed} ✔ · ${res.failed.length} ✘`, ...res.failed.map((f) => el("p", {}, el("code", {}, f)))),
    el("pre", { class: "mono", style: "white-space:pre-wrap;font-size:12px" }, res.log.join("\n")),
  );
});

$("set-height").addEventListener("click", async () => {
  snapshotCache = await engine({ op: "height", height: Number($("height").value) || 1 });
  renderState();
});
$("advance").addEventListener("click", async () => {
  const snap = snapshotCache || (await engine({ op: "snapshot" }));
  snapshotCache = await engine({ op: "height", height: snap.height + 20 });
  renderState();
});
$("export-session").addEventListener("click", async () => {
  const res = await engine({ op: "export" });
  download("tccl-session.json", JSON.stringify({ contracts, state: res.state }), "application/json");
  notice(S.sessionExported);
});
$("import-session").addEventListener("click", () => $("session-file").click());
$("session-file").addEventListener("change", (e) => {
  const f = e.target.files[0];
  if (!f) return;
  const reader = new FileReader();
  reader.onload = async () => {
    try {
      const data = JSON.parse(String(reader.result));
      const res = await engine({ op: "import", state: data.state });
      if (res.fatal) return notice(res.fatal, true);
      contracts = [];
      for (const hex of Object.keys(res.contracts)) {
        const known = (data.contracts || []).find((c) => c.hex === hex);
        if (known) contracts.push(known);
      }
      snapshotCache = res;
      renderContracts();
    } catch (err) {
      notice(String(err), true);
    }
  };
  reader.readAsText(f);
});
$("reset-session").addEventListener("click", async () => {
  if (!confirm(S.confirmReset)) return;
  snapshotCache = await engine({ op: "reset" });
  contracts = [];
  $("activity").replaceChildren();
  renderContracts();
});

// ---------------- Examples ----------------
let examples = [];
$("example").addEventListener("change", async () => {
  const ex = examples.find((x) => x.name === $("example").value);
  if (!ex) return;
  src.value = ex.source;
  onInput();
  const scenarioName = ex.name.replace(/\.tccl$/, ".scenario");
  try {
    const r = await fetch(`/examples/${scenarioName}`);
    $("scenario").value = r.ok ? await r.text() : `deploy ${ex.name} as app\n`;
  } catch {
    $("scenario").value = `deploy ${ex.name} as app\n`;
  }
});

// ---------------- Start ----------------
(async () => {
  startWorker();
  const version = await ready;
  if (version.fatal) {
    setStatus(version.fatal, "bad");
    return;
  }
  setStatus(`${S.ready} · tccl ${version.tccl}`);
  const list = await engine({ op: "examples" });
  examples = list.examples;
  $("example").append(...examples.map((x) => el("option", { value: x.name }, x.name)));
  const saved = storage.get("tccl.playground.source");
  src.value = saved || examples[0].source;
  if (!saved) {
    $("example").value = examples[0].name;
    $("scenario").value = await fetch("/examples/counter.scenario").then((r) => (r.ok ? r.text() : "")).catch(() => "");
  }
  onInput();
  renderState();
})();
