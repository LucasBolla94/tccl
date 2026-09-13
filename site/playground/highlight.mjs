// TCCL syntax highlighting shared by the documentation build (Node) and the
// playground editor (browser). Produces escaped HTML with <span class="t-…">.

const KEYWORDS = new Set(
  "contract module const state event init action view fn payable let if elif else while for in break continue return require send emit destroy pass and or not use as record enum interface role only grant revoke to from with upgrade".split(
    " ",
  ),
);
const TYPES = new Set("int bool text bytes address list map".split(" "));
const CONSTANTS = new Set(["true", "false", "TCN"]);
const CONTEXT = new Set(["caller", "value", "balance", "height", "self", "origin"]);
const BUILTINS = new Set(
  "len sha256 blake3 to_bytes to_text to_int min max abs slice verify_ed25519 ring_verify address_of zero_address address range mul_div isqrt pow code_hash is_contract is_final".split(
    " ",
  ),
);

export function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

const TOKEN =
  /(#[^\n]*)|("(?:[^"\\\n]|\\.)*"?)|(0x[0-9a-fA-F_]*)|(\d[\d_]*(?:\.\d+)?(?:tcn)?)|([A-Za-z_][A-Za-z0-9_]*)|(->|==|!=|<=|>=|\+=|-=|\*=|[-+*\/%<>=:,.()\[\]{}])|(\s+)|([^\s])/g;

export function highlight(code) {
  let out = "";
  let prev = "";
  for (const m of code.matchAll(TOKEN)) {
    const [text, comment, str, hex, num, word, op] = m;
    const e = escapeHtml(text);
    if (comment) out += `<span class="t-comment">${e}</span>`;
    else if (str) out += `<span class="t-string">${e}</span>`;
    else if (hex || num) out += `<span class="t-number">${e}</span>`;
    else if (word) {
      let cls = "";
      if (KEYWORDS.has(word)) cls = "t-keyword";
      else if (TYPES.has(word)) cls = "t-type";
      else if (CONSTANTS.has(word)) cls = "t-constant";
      else if (CONTEXT.has(word)) cls = "t-context";
      else if (BUILTINS.has(word)) cls = "t-builtin";
      else if (/^[A-Z]/.test(word)) cls = "t-name";
      else if (["action", "view", "fn", "event", "record", "enum", "interface", "role", "contract", "module"].includes(prev)) cls = "t-def";
      out += cls ? `<span class="${cls}">${e}</span>` : e;
    } else if (op) out += `<span class="t-op">${e}</span>`;
    else out += e;
    if (!/^\s+$/.test(text)) prev = text;
  }
  return out;
}
