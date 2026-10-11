import { copyFile, mkdir, readFile, readdir, stat } from "node:fs/promises";
import { resolve } from "node:path";

const bundleDirectory = resolve("src-tauri", "target", "x86_64-pc-windows-msvc", "release", "bundle", "nsis");
const config = JSON.parse(await readFile(resolve("src-tauri", "tauri.conf.json"), "utf8"));
const expectedSuffix = `_${config.version}_x64-setup.exe`;
const sourceName = (await readdir(bundleDirectory)).find((name) => name.startsWith("SeekDash_") && name.endsWith(expectedSuffix));

if (!sourceName) {
  throw new Error(`SeekDash NSIS output was not found in ${bundleDirectory}`);
}

const source = resolve(bundleDirectory, sourceName);
const outputDirectory = resolve("release");
const destination = resolve(outputDirectory, `SeekDash_Setup_${config.version}_x64.exe`);

await stat(source);
await mkdir(outputDirectory, { recursive: true });
await copyFile(source, destination);
console.log(`SeekDash installer copied to ${destination}`);
