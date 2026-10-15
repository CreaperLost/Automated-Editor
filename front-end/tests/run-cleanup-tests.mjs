import { build } from "esbuild";
import { spawnSync } from "node:child_process";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const tests = (await readdir(here)).filter((name) => name.endsWith(".test.ts"));
const directory = await mkdtemp(join(tmpdir(), "aeroedits-cleanup-tests-"));
try {
  const outfiles = [];
  for (const name of tests) {
    const outfile = join(directory, name.replace(/\.ts$/, ".cjs"));
    await build({ entryPoints: [join(here, name)], outfile, bundle: true, platform: "node", format: "cjs", logLevel: "silent" });
    outfiles.push(outfile);
  }
  const result = spawnSync(process.execPath, ["--test", ...outfiles], { stdio: "inherit" });
  process.exitCode = result.status ?? 1;
} finally {
  await rm(directory, { recursive: true, force: true });
}
