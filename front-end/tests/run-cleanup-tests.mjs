import { build } from "esbuild";
import { spawnSync } from "node:child_process";
import { mkdtemp, rm, rmdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const directory = await mkdtemp(join(tmpdir(), "aeroedits-cleanup-tests-"));
const outfile = join(directory, "cleanup.cjs");
try {
  await build({ entryPoints: [fileURLToPath(new URL("./jumpCutStore.test.ts", import.meta.url))], outfile, bundle: true, platform: "node", format: "cjs", logLevel: "silent" });
  const result = spawnSync(process.execPath, ["--test", outfile], { stdio: "inherit" });
  process.exitCode = result.status ?? 1;
} finally {
  await rm(outfile, { force: true });
  await rmdir(directory);
}
