// Screenshots of the preview build (`npm run build:preview`, mocked backend) at phone and
// desktop widths: the start screen, the editor with the canned project, and the Export dialog.
//
//   node scripts/preview-screenshots.mjs [out-dir=preview-screenshots] [build-dir=dist-preview]
//
// Needs Playwright with Chromium (`npm i --no-save playwright && npx playwright install chromium`).
// Fails when the page throws or the mocked backend is asked something it has no answer for,
// so a new command shows up here before it shows up as a blank panel.
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, resolve } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");

const out = resolve(process.argv[2] ?? "preview-screenshots");
const root = resolve(process.argv[3] ?? "dist-preview");
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml",
  ".png": "image/png", ".jpg": "image/jpeg", ".woff2": "font/woff2", ".json": "application/json" };

const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
  const file = join(root, path === "/" ? "index.html" : path);
  try {
    const body = await readFile(file);
    res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;
await mkdir(out, { recursive: true });

const sizes = [
  { name: "phone", viewport: { width: 390, height: 844 } },
  { name: "desktop", viewport: { width: 1440, height: 900 } },
];
const problems = [];
const browser = await chromium.launch();
try {
  for (const { name, viewport } of sizes) {
    const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
    page.on("pageerror", (e) => problems.push(`${name}: page error: ${e.message}`));
    page.on("console", (m) => {
      const text = m.text();
      if (text.startsWith("[mock]") || m.type() === "error") problems.push(`${name}: ${text}`);
    });
    const shot = async (step) => {
      await page.waitForTimeout(600);
      const file = join(out, `${name}-${step}.png`);
      await page.screenshot({ path: file });
      console.log(file);
    };
    try {
    await page.goto(url);
    // The canned project, listed as a recent one.
    const recent = page.getByRole("button", { name: /AeroEdits\/Smoke/ }).first();
    await recent.waitFor();
    await shot("1-start");
    await recent.click();
    await page.getByRole("button", { name: /export/i }).first().waitFor({ timeout: 15000 });
    await shot("2-editor");
    await page.getByRole("button", { name: /export/i }).first().click();
    await shot("3-export");
    } catch (error) {
      problems.push(`${name}: ${error.message.split("\n")[0]}`);
      await shot("failed");
    }
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
const unique = [...new Set(problems)];
if (unique.length) {
  console.error(`\n${unique.length} problem(s):\n${unique.join("\n")}`);
  process.exit(1);
}
