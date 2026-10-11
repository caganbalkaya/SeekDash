import { copyFile, stat } from "node:fs/promises";
import { resolve } from "node:path";

const source = resolve("src-tauri", "target", "release", "seekdash.exe");
const destination = resolve("SeekDash.exe");

await stat(source);
await copyFile(source, destination);
console.log(`SeekDash launcher copied to ${destination}`);
