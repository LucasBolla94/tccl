// Checks every internal link and #anchor of the built site (run after `node site/build.mjs`).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const dist = path.join(path.dirname(fileURLToPath(import.meta.url)), "../dist");
const pages = new Map();
function walk(dir) {
  for (const f of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, f.name);
    if (f.isDirectory()) walk(p);
    else if (p.endsWith(".html")) pages.set("/" + path.relative(dist, p).split(path.sep).join("/"), fs.readFileSync(p, "utf8"));
  }
}
walk(dist);
const ids = new Map([...pages].map(([p, html]) => [p, new Set([...html.matchAll(/ id="([^"]+)"/g)].map((m) => m[1]))]));
const problems = [];
for (const [page, html] of pages) {
  for (const [, href] of html.matchAll(/ href="([^"]+)"/g)) {
    if (/^(https?:|mailto:)/.test(href)) continue;
    let [target, anchor] = href.split("#");
    if (!target) target = page;
    else if (!target.startsWith("/")) target = path.posix.normalize(path.posix.join(path.posix.dirname(page), target));
    target = target.split("?")[0];
    if (target.endsWith("/")) target += "index.html";
    const file = path.join(dist, target);
    if (!pages.has(target) && !fs.existsSync(file)) {
      problems.push(`${page}: broken link ${href}`);
      continue;
    }
    if (anchor && pages.has(target) && !ids.get(target).has(anchor)) problems.push(`${page}: missing anchor ${href}`);
  }
}
if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`${pages.size} pages, all internal links and anchors resolve`);
