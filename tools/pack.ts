// GitHub Releaseへ添付する配布物を、パッケージごとのtarballとして dist/ に作る。
//
// 版はルートのpackage.jsonだけが持ち、各パッケージのpackage.jsonには書かない。
// 詰める直前に複製へ版を書き込み、workspace:* を同じ版へ置き換えるため、全パッケージが常に同じ版で揃う。
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const out = join(root, "dist");

type Manifest = {
  version?: string;
  workspaces?: string[];
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8")) as Manifest;
const version = manifest.version;
if (!version) throw new Error("ルートのpackage.jsonにversionが無い");
const tag = process.env.YOMILO_RELEASE_TAG;
if (tag !== undefined && tag !== `v${version}`) {
  throw new Error(`タグ ${tag} がルートのpackage.jsonの版 v${version} と一致しない`);
}

const directories = (manifest.workspaces ?? [])
  .flatMap((pattern) => [...new Bun.Glob(pattern).scanSync({ cwd: root, onlyFiles: false })])
  .filter((directory) => existsSync(join(root, directory, "package.json")))
  .sort();
if (directories.length === 0) throw new Error("配布するパッケージが無い");

const wasm = join(root, "crates/yomilo-wasm/pkg/yomilo_wasm_bg.wasm");
if (!existsSync(wasm)) throw new Error("WASMが未生成。先に just wasm-build を実行する");

function pin(deps: Record<string, string> | undefined): void {
  if (!deps) return;
  for (const [name, range] of Object.entries(deps)) {
    if (range.startsWith("workspace:")) deps[name] = version as string;
  }
}

rmSync(out, { recursive: true, force: true });
mkdirSync(out);
const staging = mkdtempSync(join(tmpdir(), "yomilo-pack-"));
try {
  for (const directory of directories) {
    const target = join(staging, directory);
    cpSync(join(root, directory), target, {
      recursive: true,
      filter: (path) => !path.includes("/node_modules"),
    });
    cpSync(join(root, "LICENSE"), join(target, "LICENSE"));
    const { name, ...rest } = JSON.parse(
      readFileSync(join(target, "package.json"), "utf8"),
    ) as Manifest & { name: string };
    const body: Manifest = { name, version, ...rest } as Manifest;
    pin(body.dependencies);
    pin(body.peerDependencies);
    writeFileSync(join(target, "package.json"), `${JSON.stringify(body, null, 2)}\n`);
    const result = Bun.spawnSync(["bun", "pm", "pack", "--quiet", "--destination", out], {
      cwd: target,
      stderr: "pipe",
    });
    if (result.exitCode !== 0) {
      throw new Error(`${directory} を詰められなかった:\n${result.stderr.toString()}`);
    }
    console.log(result.stdout.toString().trim());
  }
} finally {
  rmSync(staging, { recursive: true, force: true });
}
