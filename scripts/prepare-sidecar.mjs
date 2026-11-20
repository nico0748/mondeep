#!/usr/bin/env node
// MCP サーバーをリリースビルドし、Tauri の sidecar（externalBin）の置き場所にコピーする。
// Tauri は binaries/mondeep-mcp-<ターゲットの三つ組> を探し、バンドル時にアプリ本体と同じフォルダへ
// 「mondeep-mcp」という名前で入れる。アプリの設定画面はそこを MCP サーバーとして案内する。
//
// 使い方: node scripts/prepare-sidecar.mjs [--target <triple>]
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, chmodSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const targetArg = args.indexOf("--target") >= 0 ? args[args.indexOf("--target") + 1] : process.env.TAURI_ENV_TARGET_TRIPLE;

const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(/^host: (.+)$/m)[1].trim();
const target = targetArg || host;
const ext = target.includes("windows") ? ".exe" : "";

const cargoArgs = ["build", "--release", "-p", "mondeep-mcp"];
if (targetArg) cargoArgs.push("--target", target);
console.log(`> cargo ${cargoArgs.join(" ")}`);
execFileSync("cargo", cargoArgs, { cwd: root, stdio: "inherit" });

const built = join(root, "target", ...(targetArg ? [target] : []), "release", `mondeep-mcp${ext}`);
const destDir = join(root, "app", "src-tauri", "binaries");
mkdirSync(destDir, { recursive: true });
const dest = join(destDir, `mondeep-mcp-${target}${ext}`);
copyFileSync(built, dest);
if (!ext) chmodSync(dest, 0o755);
console.log(`sidecar: ${dest}`);
