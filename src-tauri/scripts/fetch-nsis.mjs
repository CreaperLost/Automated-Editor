#!/usr/bin/env node
// Puts the NSIS toolset that `tauri build --bundles nsis` needs into Tauri's tools folder
// (%LOCALAPPDATA%\tauri\NSIS), so the bundler finds it and downloads nothing. The bundler
// fetches it from GitHub release links with no retry, and those answered 504 for hours on
// 8 Nov; this script uses the link-then-API download in github-release.mjs.
//
//   node scripts/fetch-nsis.mjs [--force]
//
// Windows only. The URLs, SHA-1 sums and required files mirror tauri-bundler's
// bundle/windows/nsis/mod.rs at tag tauri-cli-v2.12.4, the CLI version package.yml pins.
// When that pin moves, compare this list with the new tag's file.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { downloadReleaseAsset } from "./github-release.mjs";

const NSIS = {
  repo: "tauri-apps/binary-releases",
  tag: "nsis-3.13",
  asset: "nsis-3.13.zip",
  sha1: "db14f8af2ea346b786c6ae19343f61aaf5e09b8e",
  folder: "nsis-3.13",
};
const NSIS_EXE_SHA1 = "6d45d4e8ccf18fe1fdac9164058f356102a2ecf2";
const UTILS = {
  repo: "tauri-apps/nsis-tauri-utils",
  tag: "nsis_tauri_utils-v0.5.3",
  asset: "nsis_tauri_utils.dll",
  sha1: "75197fee3c6a814fe035788d1c34ead39349b860",
  file: "Plugins/x86-unicode/additional/nsis_tauri_utils.dll",
};
const REQUIRED_FILES = [
  "makensis.exe",
  "Bin/makensis.exe",
  "Stubs/lzma-x86-unicode",
  "Stubs/lzma_solid-x86-unicode",
  UTILS.file,
  "Include/MUI2.nsh",
  "Include/FileFunc.nsh",
  "Include/x64.nsh",
  "Include/nsDialogs.nsh",
  "Include/WinMessages.nsh",
  "Include/Win/COM.nsh",
  "Include/Win/Propkey.nsh",
  "Include/Win/RestartManager.nsh",
];

const sha1 = (data) => createHash("sha1").update(data).digest("hex");
const fileSha1 = (file) => (fs.existsSync(file) ? sha1(fs.readFileSync(file)) : undefined);

async function fetchVerified({ repo, tag, asset, sha1: expected }) {
  console.log(`Downloading ${asset}`);
  const data = await downloadReleaseAsset(repo, tag, asset);
  const actual = sha1(data);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${asset}: expected ${expected}, got ${actual}.`);
  }
  return data;
}

async function main() {
  if (process.platform !== "win32") throw new Error("fetch-nsis.mjs runs on Windows only.");
  if (!process.env.LOCALAPPDATA) throw new Error("LOCALAPPDATA is not set.");
  const tools = path.join(process.env.LOCALAPPDATA, "tauri");
  const nsis = path.join(tools, "NSIS");

  const ready = () =>
    REQUIRED_FILES.every((f) => fs.existsSync(path.join(nsis, f))) &&
    fileSha1(path.join(nsis, "NSIS.exe")) === NSIS_EXE_SHA1 &&
    fileSha1(path.join(nsis, UTILS.file)) === UTILS.sha1;
  if (!process.argv.includes("--force") && ready()) {
    console.log(`NSIS is already in ${nsis} (use --force to refresh).`);
    return;
  }

  const zip = await fetchVerified(NSIS);
  const dll = await fetchVerified(UTILS);

  // Unpacked beside the target so the final rename stays on one drive.
  fs.mkdirSync(tools, { recursive: true });
  const work = fs.mkdtempSync(path.join(tools, "nsis-fetch-"));
  try {
    const zipPath = path.join(work, NSIS.asset);
    fs.writeFileSync(zipPath, zip);
    // Windows 10+ ships bsdtar, which unpacks zip files; Git Bash's tar can come first on PATH.
    const tar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
    execFileSync(tar, ["-xf", zipPath, "-C", work], { stdio: "inherit" });
    const unpacked = path.join(work, NSIS.folder);
    const dllPath = path.join(unpacked, UTILS.file);
    fs.mkdirSync(path.dirname(dllPath), { recursive: true });
    fs.writeFileSync(dllPath, dll);

    fs.rmSync(nsis, { recursive: true, force: true });
    fs.renameSync(unpacked, nsis);
  } finally {
    fs.rmSync(work, { recursive: true, force: true });
  }
  if (!ready()) throw new Error(`${nsis} is still missing files or has the wrong NSIS.exe.`);
  console.log(`NSIS is ready in ${nsis}`);
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
