#!/usr/bin/env node
// Builds https://tccl.the-coin.cloud from content (docs/, site/content/) and design (site/theme/).
// No dependencies: `node site/build.mjs [--wasm path/to/tccl_wasm.wasm] [--out site/dist]`.

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";
import { highlight, escapeHtml } from "./playground/highlight.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");
const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const out = path.resolve(opt("--out", path.join(here, "dist")));
const wasmPath = path.resolve(opt("--wasm", path.join(root, "target/wasm32-unknown-unknown/wasm/tccl_wasm.wasm")));
const siteUrl = "https://tccl.the-coin.cloud";

const site = JSON.parse(fs.readFileSync(path.join(here, "content/site.json"), "utf8"));
const layout = fs.readFileSync(path.join(here, "theme/layout.html"), "utf8");
const version = JSON.parse(fs.readFileSync(path.join(here, "content/version.json"), "utf8"));
const read = (p) => fs.readFileSync(p, "utf8");
const write = (p, s) => {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, s);
};
const hash = (buf) => crypto.createHash("sha256").update(buf).digest("hex").slice(0, 10);

// ---------- Markdown (the subset used by the documentation) ----------

function slugify(s) {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/<[^>]+>/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

function inline(s) {
  const codes = [];
  s = s.replace(/`([^`]+)`/g, (_, c) => {
    codes.push(`<code>${escapeHtml(c)}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  s = escapeHtml(s);
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text, href) => {
    let h = href.replace(/\.md(#|$)/, ".html$1");
    const ext = /^https?:/.test(h) ? ' rel="noopener"' : "";
    return `<a href="${h}"${ext}>${text}</a>`;
  });
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>").replace(/(^|[^*\w])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>");
  return s.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[+i]);
}

function codeBlock(lang, code, label) {
  const body = lang === "tccl" || lang === "scenario" ? highlight(code) : escapeHtml(code);
  const l = label ? `<span class="label">${escapeHtml(label)}</span>` : "";
  return `<pre class="code" data-lang="${lang || "text"}">${l}<code>${body}</code></pre>`;
}

function markdown(src, headings) {
  src = src.replace(/\{\{(example|scenario):([\w.-]+)\}\}/g, (_, kind, file) => {
    const text = read(path.join(root, "examples", file)).replace(/\n$/, "");
    return "```" + (kind === "example" ? "tccl" : "scenario") + " " + file + "\n" + text + "\n```";
  });
  src = src.replace(/\{\{include:([\w./-]+)\}\}/g, (_, file) => read(path.join(root, file)));
  const lines = src.split("\n");
  let html = "";
  let i = 0;
  let para = [];
  const flush = () => {
    if (para.length) html += `<p>${inline(para.join(" "))}</p>\n`;
    para = [];
  };
  while (i < lines.length) {
    const line = lines[i];
    const fence = line.match(/^```(\w*)\s*(.*)$/);
    if (fence) {
      flush();
      const body = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) body.push(lines[i++]);
      i++;
      html += codeBlock(fence[1], body.join("\n"), fence[2]) + "\n";
      continue;
    }
    const h = line.match(/^(#{1,4})\s+(.*)$/);
    if (h) {
      flush();
      const level = h[1].length;
      const explicit = h[2].match(/\s*\{#([\w-]+)\}\s*$/);
      const text = explicit ? h[2].slice(0, explicit.index) : h[2];
      const id = explicit ? explicit[1] : slugify(text);
      if (level === 2 || level === 3) headings.push({ level, text, id });
      html += level === 1 ? `<h1>${inline(text)}</h1>\n` : `<h${level} id="${id}">${inline(text)}<a class="anchor" href="#${id}" aria-hidden="true">#</a></h${level}>\n`;
      i++;
      continue;
    }
    if (/^\|.*\|\s*$/.test(line) && i + 1 < lines.length && /^\|[\s:|-]+\|\s*$/.test(lines[i + 1])) {
      flush();
      const cells = (l) =>
        l
          .trim()
          .replace(/^\||\|$/g, "")
          .split(/(?<!\\)\|/)
          .map((c) => inline(c.trim().replace(/\\\|/g, "|")));
      const head = cells(line);
      i += 2;
      let rows = "";
      while (i < lines.length && /^\|.*\|\s*$/.test(lines[i])) rows += `<tr>${cells(lines[i++]).map((c) => `<td>${c}</td>`).join("")}</tr>`;
      html += `<div class="table-scroll"><table><thead><tr>${head.map((c) => `<th>${c}</th>`).join("")}</tr></thead><tbody>${rows}</tbody></table></div>\n`;
      continue;
    }
    if (/^>\s?/.test(line)) {
      flush();
      const body = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) body.push(lines[i++].replace(/^>\s?/, ""));
      html += `<blockquote>${markdown(body.join("\n"), [])}</blockquote>\n`;
      continue;
    }
    if (/^\s*([-*]|\d+\.)\s+/.test(line)) {
      flush();
      const ordered = /^\s*\d+\./.test(line);
      const items = [];
      while (i < lines.length && (/^\s*([-*]|\d+\.)\s+/.test(lines[i]) || (/^\s{2,}\S/.test(lines[i]) && items.length))) {
        if (/^\s*([-*]|\d+\.)\s+/.test(lines[i]) && !/^\s{2,}/.test(lines[i])) items.push(lines[i].replace(/^\s*([-*]|\d+\.)\s+/, ""));
        else items[items.length - 1] += "\n" + lines[i].trim().replace(/^[-*]\s+/, "• ");
        i++;
      }
      const tag = ordered ? "ol" : "ul";
      html += `<${tag}>${items.map((t) => `<li>${inline(t).replace(/\n/g, "<br>")}</li>`).join("")}</${tag}>\n`;
      continue;
    }
    if (/^---\s*$/.test(line)) {
      flush();
      html += "<hr>\n";
      i++;
      continue;
    }
    if (line.trim() === "") {
      flush();
      i++;
      continue;
    }
    if (/^<[a-z]/.test(line.trim())) {
      flush();
      html += line + "\n";
      i++;
      continue;
    }
    para.push(line.trim());
    i++;
  }
  flush();
  return html;
}

// ---------- Pages ----------

const langs = site.languages;
const prefix = (lang) => (lang.dir ? `/${lang.dir}` : "");
const t = (lang, key) => site.strings[lang.code][key] ?? site.strings.en[key] ?? key;
const pages = [];

function page(lang, { pathName, title, description, content, current, scripts = "", alternates }) {
  const nav = site.nav
    .map((n) => `<a href="${prefix(lang)}${n.href}"${current === n.id ? ' aria-current="page"' : ""}>${escapeHtml(t(lang, n.label))}</a>`)
    .join("");
  const switcher = langs
    .map((l) => `<a href="${alternates[l.code]}" hreflang="${l.code}" lang="${l.code}"${l.code === lang.code ? ' aria-current="true"' : ""}>${l.short}</a>`)
    .join("");
  const hreflang = langs.map((l) => `<link rel="alternate" hreflang="${l.code}" href="${siteUrl}${alternates[l.code]}">`).join("");
  const html = layout
    .replaceAll("{{lang}}", lang.code)
    .replaceAll("{{title}}", escapeHtml(title))
    .replaceAll("{{description}}", escapeHtml(description))
    .replaceAll("{{canonical}}", `${siteUrl}${alternates[lang.code]}`)
    .replaceAll("{{hreflang}}", hreflang)
    .replaceAll("{{home}}", `${prefix(lang)}/`)
    .replaceAll("{{nav}}", nav)
    .replaceAll("{{langSwitch}}", switcher)
    .replaceAll("{{skip}}", escapeHtml(t(lang, "skip")))
    .replaceAll("{{menu}}", escapeHtml(t(lang, "menu")))
    .replaceAll("{{footer}}", footer(lang))
    .replaceAll("{{version}}", version.tool)
    .replaceAll("{{css}}", `/assets/style.css?v=${assetVersion}`)
    .replaceAll("{{scripts}}", scripts)
    .replace("{{content}}", content);
  write(path.join(out, pathName), html);
  pages.push(alternates[lang.code]);
}

function footer(lang) {
  const links = site.footer.map((n) => `<a href="${n.href.startsWith("http") ? n.href : prefix(lang) + n.href}">${escapeHtml(t(lang, n.label))}</a>`).join("");
  return `<div class="wrap footer-inner"><span>${escapeHtml(t(lang, "footer"))} · tccl ${version.tool} · ${escapeHtml(t(lang, "languageVersion"))} ${version.language}</span><nav>${links}</nav></div>`;
}

let assetVersion = "0";

function build() {
  fs.rmSync(out, { recursive: true, force: true });
  // Design
  fs.cpSync(path.join(here, "theme/assets"), path.join(out, "assets"), { recursive: true });
  assetVersion = hash(read(path.join(here, "theme/assets/style.css")) + read(path.join(here, "playground/playground.js")));
  for (const f of ["highlight.mjs", "playground.js", "worker.js"]) fs.copyFileSync(path.join(here, "playground", f), path.join(out, "assets", f));
  // Engine
  if (!fs.existsSync(wasmPath)) throw new Error(`WebAssembly engine not found at ${wasmPath}; build it with: cargo build -p tccl-wasm --target wasm32-unknown-unknown --profile wasm`);
  const wasm = fs.readFileSync(wasmPath);
  const wasmName = `tccl-${hash(wasm)}.wasm`;
  write(path.join(out, "engine", wasmName), wasm);
  // Raw examples, installers
  fs.cpSync(path.join(root, "examples"), path.join(out, "examples"), { recursive: true });
  for (const f of ["install.sh", "install.ps1"]) fs.copyFileSync(path.join(root, "installer", f), path.join(out, f));

  const docs = site.docs;
  for (const lang of langs) {
    const strings = site.strings[lang.code];
    // Home
    const home = JSON.parse(read(path.join(here, `content/home/${lang.code}.json`)));
    const alternatesHome = Object.fromEntries(langs.map((l) => [l.code, `${prefix(l)}/`]));
    page(lang, {
      pathName: `${lang.dir ? lang.dir + "/" : ""}index.html`,
      title: home.title,
      description: home.description,
      current: "home",
      alternates: alternatesHome,
      content: renderHome(lang, home),
    });
    // Docs
    const titles = docs.map((slug) => {
      const md = read(path.join(root, "docs", lang.code, `${slug}.md`));
      return (md.match(/^#\s+(.*)$/m) || [null, slug])[1];
    });
    docs.forEach((slug, idx) => {
      const md = read(path.join(root, "docs", lang.code, `${slug}.md`));
      const headings = [];
      let body = markdown(md, headings);
      body = body.replace(/href="(?!https?:|#|\/)([\w-]+)\.html/g, `href="${prefix(lang)}/docs/$1.html`);
      body = body.replace(/href="\.\.\/\.\.\/examples\//g, `href="/examples/`);
      const sidebar = `<nav class="doc-nav" aria-label="${escapeHtml(strings.docsNav)}"><h2>${escapeHtml(strings.docs)}</h2><ol>${docs
        .map((s, j) => `<li><a href="${prefix(lang)}/docs/${s}.html"${s === slug ? ' aria-current="page"' : ""}>${escapeHtml(titles[j])}</a></li>`)
        .join("")}</ol>${
        headings.filter((h) => h.level === 2).length > 2
          ? `<h2>${escapeHtml(strings.onThisPage)}</h2><ol>${headings
              .filter((h) => h.level === 2)
              .map((h) => `<li><a href="#${h.id}">${escapeHtml(h.text.replace(/`/g, ""))}</a></li>`)
              .join("")}</ol>`
          : ""
      }</nav>`;
      const prev = idx > 0 ? `<a class="btn" href="${prefix(lang)}/docs/${docs[idx - 1]}.html">← ${escapeHtml(titles[idx - 1])}</a>` : "<span></span>";
      const next = idx < docs.length - 1 ? `<a class="btn" href="${prefix(lang)}/docs/${docs[idx + 1]}.html">${escapeHtml(titles[idx + 1])} →</a>` : "<span></span>";
      const edit = `<p class="note"><a href="https://github.com/LucasBolla94/tccl/blob/main/docs/${lang.code}/${slug}.md">${escapeHtml(strings.editPage)}</a></p>`;
      page(lang, {
        pathName: `${lang.dir ? lang.dir + "/" : ""}docs/${slug}.html`,
        title: `${titles[idx]} — TCCL`,
        description: (md.split("\n").find((l) => l.trim() && !l.startsWith("#") && !l.startsWith(">")) || titles[idx]).slice(0, 180),
        current: "docs",
        alternates: Object.fromEntries(langs.map((l) => [l.code, `${prefix(l)}/docs/${slug}.html`])),
        content: `<div class="wrap doc-layout">${sidebar}<article class="doc">${body}${edit}<div class="pager">${prev}${next}</div></article></div>`,
      });
    });
    // Playground
    const pg = JSON.parse(read(path.join(here, `content/playground/${lang.code}.json`)));
    const pgHtml = read(path.join(here, "theme/playground.html")).replace(/\{\{(\w+)\}\}/g, (_, k) => {
      if (k === "docs") return `${prefix(lang)}/docs`;
      if (!(k in pg)) throw new Error(`playground string '${k}' missing for ${lang.code}`);
      return escapeHtml(pg[k]);
    });
    page(lang, {
      pathName: `${lang.dir ? lang.dir + "/" : ""}playground/index.html`,
      title: pg.pageTitle,
      description: pg.pageDescription,
      current: "playground",
      alternates: Object.fromEntries(langs.map((l) => [l.code, `${prefix(l)}/playground/`])),
      content: pgHtml,
      scripts: `<script type="module" src="/assets/playground.js?v=${assetVersion}" data-wasm="/engine/${wasmName}" data-lang="${lang.code}" data-strings="${escapeHtml(JSON.stringify(pg))}" id="playground-script"></script>`,
    });
    // Download
    const dl = read(path.join(here, `content/download/${lang.code}.md`)).replaceAll("{{version}}", version.tool);
    const dlHeadings = [];
    page(lang, {
      pathName: `${lang.dir ? lang.dir + "/" : ""}download/index.html`,
      title: `${(dl.match(/^#\s+(.*)$/m) || [null, "Download"])[1]} — TCCL`,
      description: strings.downloadDescription,
      current: "download",
      alternates: Object.fromEntries(langs.map((l) => [l.code, `${prefix(l)}/download/`])),
      content: `<div class="wrap doc-layout"><div></div><article class="doc">${markdown(dl, dlHeadings)}</article></div>`,
    });
  }
  // 404, robots, sitemap
  const en = langs[0];
  page(en, {
    pathName: "404.html",
    title: "Not found — TCCL",
    description: "Page not found",
    current: "",
    alternates: Object.fromEntries(langs.map((l) => [l.code, `${prefix(l)}/`])),
    content: `<div class="wrap" style="padding:80px 0"><h1>404</h1><p class="lead">This page does not exist. · Esta página não existe. · Esta página no existe.</p><div class="actions">${langs.map((l) => `<a class="btn" href="${prefix(l)}/">${l.name}</a>`).join("")}</div></div>`,
  });
  write(path.join(out, "robots.txt"), `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`);
  write(path.join(out, "sitemap.xml"), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${pages.filter((p) => !p.includes("404")).map((p) => `<url><loc>${siteUrl}${p}</loc></url>`).join("")}</urlset>\n`);
  console.log(`built ${pages.length} pages into ${out} (engine ${wasmName}, ${(wasm.length / 1e6).toFixed(2)} MB)`);
}

function renderHome(lang, h) {
  const p = prefix(lang);
  const cards = (items) => `<div class="grid">${items.map((c) => `<div class="card"><h3>${escapeHtml(c.title)}</h3><p>${inline(c.text)}</p></div>`).join("")}</div>`;
  return `
<section class="wrap hero">
  <div>
    <div class="eyebrow">${escapeHtml(h.eyebrow)}</div>
    <h1>${escapeHtml(h.headline)}</h1>
    <p class="lead">${inline(h.lead)}</p>
    <div class="actions">
      <a class="btn btn-primary" href="${p}/playground/">${escapeHtml(h.ctaPlayground)} →</a>
      <a class="btn" href="${p}/docs/tutorial.html">${escapeHtml(h.ctaTutorial)}</a>
      <a class="btn" href="${p}/download/">${escapeHtml(h.ctaDownload)}</a>
    </div>
    <p class="note">${inline(h.note)}</p>
  </div>
  <div>${codeBlock("tccl", read(path.join(root, "examples", h.sample)).trim(), h.sample)}</div>
</section>
<section class="section"><div class="wrap"><h2>${escapeHtml(h.beginnersTitle)}</h2><p class="lead">${inline(h.beginnersLead)}</p>${cards(h.beginners)}</div></section>
<section class="section"><div class="wrap"><h2>${escapeHtml(h.advancedTitle)}</h2><p class="lead">${inline(h.advancedLead)}</p>${cards(h.advanced)}</div></section>
<section class="section"><div class="wrap"><h2>${escapeHtml(h.safetyTitle)}</h2><p class="lead">${inline(h.safetyLead)}</p>${cards(h.safety)}</div></section>
<section class="section"><div class="wrap"><h2>${escapeHtml(h.honestTitle)}</h2>${h.honest.map((x) => `<p>${inline(x)}</p>`).join("")}<div class="actions"><a class="btn" href="${p}/docs/security.html">${escapeHtml(h.ctaSecurity)}</a><a class="btn" href="${p}/docs/versions.html">${escapeHtml(h.ctaVersions)}</a></div></div></section>`;
}

build();
