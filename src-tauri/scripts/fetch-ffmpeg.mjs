#!/usr/bin/env node
// Downloads ffmpeg and ffprobe into src-tauri/binaries/ with the names Tauri expects for
// sidecars (`ffmpeg-<target-triple>[.exe]`), so `tauri.ffmpeg.conf.json` can bundle them.
//
//   node scripts/fetch-ffmpeg.mjs [--target <triple>] [--gpl] [--force]
//
// Builds come from BtbN/FFmpeg-Builds (release branch 8.1) and are checked against the
// release's published SHA-256 sums. The default LGPL build keeps the app's MIT/Apache
// license unaffected; `--gpl` fetches the GPL build instead (adds libx264), which must then
// be distributed under the GPL's terms. macOS has no BtbN build yet: on a Mac the Swift
// bridges are the default backend and FFmpeg comes from Homebrew.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RELEASE = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";
const BRANCH = "8.1";

const PLATFORMS = {
  "x86_64-pc-windows-msvc": { build: "win64", ext: "zip", exe: ".exe" },
  "aarch64-pc-windows-msvc": { build: "winarm64", ext: "zip", exe: ".exe" },
  "x86_64-unknown-linux-gnu": { build: "linux64", ext: "tar.xz", exe: "" },
  "aarch64-unknown-linux-gnu": { build: "linuxarm64", ext: "tar.xz", exe: "" },
};

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};

function hostTriple() {
  const info = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const line = info.split(/\r?\n/).find((l) => l.startsWith("host:"));
  if (!line) throw new Error("Could not read the host target from `rustc -vV`.");
  return line.slice("host:".length).trim();
}

async function download(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Download failed (${response.status}): ${url}`);
  return Buffer.from(await response.arrayBuffer());
}

function findFile(dir, name) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findFile(full, name);
      if (found) return found;
    } else if (entry.name === name) {
      return full;
    }
  }
  return undefined;
}

async function main() {
  const triple = option("--target") ?? hostTriple();
  const platform = PLATFORMS[triple];
  if (!platform) {
    throw new Error(
      `No bundled FFmpeg for ${triple}. Supported: ${Object.keys(PLATFORMS).join(", ")}.`,
    );
  }
  const license = flag("--gpl") ? "gpl" : "lgpl";
  const asset = `ffmpeg-n${BRANCH}-latest-${platform.build}-${license}-${BRANCH}.${platform.ext}`;

  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const outDir = path.join(root, "binaries");
  const targets = ["ffmpeg", "ffprobe"].map((base) => ({
    from: `${base}${platform.exe}`,
    to: path.join(outDir, `${base}-${triple}${platform.exe}`),
  }));
  const stamp = path.join(outDir, "FFMPEG-SOURCE.txt");
  const stampText = `${RELEASE}/${asset}\n`;
  if (
    !flag("--force") &&
    targets.every((t) => fs.existsSync(t.to)) &&
    fs.existsSync(stamp) &&
    fs.readFileSync(stamp, "utf8").startsWith(stampText)
  ) {
    console.log(`FFmpeg for ${triple} is already in ${outDir} (use --force to refresh).`);
    return;
  }

  console.log(`Downloading ${asset}`);
  const sums = (await download(`${RELEASE}/checksums.sha256`)).toString("utf8");
  const expected = sums
    .split(/\r?\n/)
    .map((line) => line.trim().split(/\s+/))
    .find(([, name]) => name === asset)?.[0];
  if (!expected) throw new Error(`${asset} is not listed in the release checksums.`);
  const archive = await download(`${RELEASE}/${asset}`);
  const actual = createHash("sha256").update(archive).digest("hex");
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${asset}: expected ${expected}, got ${actual}.`);
  }

  const work = fs.mkdtempSync(path.join(os.tmpdir(), "aeroedits-ffmpeg-"));
  try {
    const archivePath = path.join(work, asset);
    fs.writeFileSync(archivePath, archive);
    if (process.platform === "win32") {
      // Windows 10+ ships bsdtar, which also unpacks zip files. Name it outright: Git Bash's
      // GNU tar can come first on PATH and cannot.
      const tar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
      execFileSync(tar, ["-xf", archivePath, "-C", work], { stdio: "inherit" });
    } else if (platform.ext === "zip") {
      execFileSync("unzip", ["-q", archivePath, "-d", work], { stdio: "inherit" });
    } else {
      execFileSync("tar", ["-xf", archivePath, "-C", work], { stdio: "inherit" });
    }

    fs.mkdirSync(outDir, { recursive: true });
    for (const { from, to } of targets) {
      const source = findFile(work, from);
      if (!source) throw new Error(`${from} is missing from ${asset}.`);
      fs.copyFileSync(source, to);
      fs.chmodSync(to, 0o755);
      console.log(`  ${path.relative(root, to)}`);
    }
    const licenseFile = findFile(work, "LICENSE.txt");
    if (!licenseFile) throw new Error(`LICENSE.txt is missing from ${asset}.`);
    fs.copyFileSync(licenseFile, path.join(outDir, "FFMPEG-LICENSE.txt"));
    fs.writeFileSync(stamp, `${stampText}sha256 ${actual}\nlicense ${license}\n`);
  } finally {
    fs.rmSync(work, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
