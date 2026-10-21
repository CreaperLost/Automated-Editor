#!/usr/bin/env node
// Downloads ffmpeg and ffprobe into src-tauri/binaries/ with the names Tauri expects for
// sidecars (`ffmpeg-<target-triple>[.exe]`), so `tauri.ffmpeg.conf.json` can bundle them.
//
//   node scripts/fetch-ffmpeg.mjs [--target <triple>] [--lgpl] [--force]
//
// Builds come from one pinned BtbN/FFmpeg-Builds release (release branch 8.1), checked against
// the SHA-256 sums written below, so every build bundles the same FFmpeg. To move to a newer
// FFmpeg, follow the review in docs/dependencies.md: pick a month-end `autobuild-*` release
// (BtbN prunes the daily ones after about two weeks), copy its asset names and sums from its
// checksums.sha256, and update RELEASE, VERSION and SHA256. The default GPL build includes
// libx264 and the NVENC, AMF and Quick Sync GPU encoders; it is a separate program the app
// runs, shipped with its license under the GPL's terms. `--lgpl` fetches the LGPL build (same
// GPU encoders, no libx264). macOS has no BtbN build: on a Mac the Swift bridges are the default backend and
// FFmpeg comes from Homebrew.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RELEASE =
  "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-30-13-08";
const BRANCH = "8.1";
const VERSION = "n8.1.3-9-g29e619e767";
// From that release's checksums.sha256.
const SHA256 = {
  "win64-gpl": "7b801cdd3a1a0bb54ae6f572187e68b4ed54f52086cfee133fab2180bfb429fa",
  "win64-lgpl": "4a7642b2264c03e8a0ce8a3825b933ee5580656f45695a086fe7e294045ffc0a",
  "winarm64-gpl": "061a770557c30bbf8e0b6ca3810288a60f72b85088378b84be03277746f34f17",
  "winarm64-lgpl": "9e49da7acb363b60bf60f9efad2ac0281cf68d43930ec4b1b8dd6a4f20dd7e5f",
  "linux64-gpl": "97ce978979194b5cf7e06a5e68020dbdaa7a4f3294c5452b6a1bc347100dbb79",
  "linux64-lgpl": "dfa863a00ca81f1bdf58a372b18cff4820f0017e55de32778de8ecd8ed92a02e",
  "linuxarm64-gpl": "10523d1e0be6b3ce62a9708cc50815c4b5eb5f36ae147fbe2afeecda6f2d6845",
  "linuxarm64-lgpl": "8463881ca7a25015ca6e00f5c91dd2f1637eb73d33c8230413cb74db760a86b3",
};

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
  const license = flag("--lgpl") ? "lgpl" : "gpl";
  const asset = `ffmpeg-${VERSION}-${platform.build}-${license}-${BRANCH}.${platform.ext}`;

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
  const expected = SHA256[`${platform.build}-${license}`];
  if (!expected) throw new Error(`No pinned checksum for ${asset}.`);
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
