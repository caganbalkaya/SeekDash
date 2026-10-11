<p align="center">
  <img src="src/assets/seekdash-logo.png" alt="SeekDash logo" width="112">
</p>

<h1 align="center">SeekDash</h1>

<p align="center"><strong>Search. Launch. Act.</strong></p>

SeekDash is a local-first launcher for Windows 10 and 11 that finds applications and files, recalls text clipboard entries, and runs a focused set of quick actions.

## Screenshots

<!-- Add screenshots captured from the signed public release here. Do not use mockups. -->

Verified release screenshots will be added before the public launch.

## Features

- Fast indexed search for applications, shortcuts, files, and folders, with exact, prefix, and fuzzy ranking.
- Native Windows application icons, favorites, recent-use ranking, and result context actions.
- Horizontal launcher and an optional left- or right-edge sidebar.
- Opt-in, searchable text Clipboard History with optional local persistence.
- A local command palette for locking, sleep, restart, shutdown, Recycle Bin, local IP addresses, UUID generation, HEX-to-RGB conversion, and Settings.
- Dark, light, and system themes with reduced-motion support.
- English, Turkish, German, and French interfaces.

## Requirements

- Windows 10 or Windows 11, x64.
- Microsoft Edge WebView2 Runtime. The installer can obtain the supported Microsoft bootstrapper when the runtime is missing.

## Download and installation

Download `SeekDash_Setup_1.0.0_x64.exe` from the [SeekDash Releases page](https://github.com/caganbalkaya/SeekDash/releases). Run the installer as the current user; administrator access is not required for normal installation.

The release should also include `SeekDash_Setup_1.0.0_x64.exe.sha256`. Verify it in PowerShell:

```powershell
Get-FileHash .\SeekDash_Setup_1.0.0_x64.exe -Algorithm SHA256
```

The installer is currently unsigned, so Windows may show a reputation warning until a trusted code-signing certificate is used.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Alt+Space` | Open or close SeekDash and focus search |
| `Alt+V` | Open or close Clipboard History |
| `Up` / `Down` | Move through results |
| `Enter` | Open or execute the selected result |
| `Esc` | Close the active launcher view |
| `>` | Enter Quick Actions mode |

Global shortcuts can be changed in Settings.

## Clipboard History

Clipboard History is disabled by default and handles plain text only. Persistent storage is a separate opt-in setting. Entries can be searched, copied, deleted individually, or cleared together. SeekDash does not claim to detect sensitive content reliably.

## Quick Actions

Type `>` to list commands, or filter directly with commands such as `>lock`, `>sleep`, `>uuid`, and `>color #FF5733`. Sleep, restart, and shutdown require confirmation. SeekDash does not execute arbitrary command-line input.

## Languages

- English
- Türkçe
- Deutsch
- Français

The saved application preference takes priority over installer and Windows language detection.

## Privacy and local data

Settings, favorites, the search index, usage history, and—when explicitly enabled—text clipboard history are stored locally in the user's application-data directory. Clipboard text is not persisted when persistent history is disabled. SeekDash has no application telemetry. External web searches and links contact the selected third-party service only when the user invokes them. See [PRIVACY.md](PRIVACY.md) for details.

## Development

Prerequisites: Node.js 20 or newer, Rust stable with the MSVC toolchain, Microsoft C++ Build Tools, and WebView2.

```powershell
npm install
npm run tauri dev
npm test
cargo test --manifest-path .\src-tauri\Cargo.toml
npm run installer
npm run checksum
```

The installer is written to `release/SeekDash_Setup_1.0.0_x64.exe`. Do not commit generated binaries, local databases, clipboard content, logs, or environment files.

## Bug reports

Use the [SeekDash issue tracker](https://github.com/caganbalkaya/SeekDash/issues). Include the SeekDash version, Windows version, reproducible steps, and the expected and actual behavior. Remove personal paths, clipboard content, and other sensitive data from logs or screenshots.

Security issues must not be reported publicly; follow [SECURITY.md](SECURITY.md).

## Website

[seekdash.com.tr](https://seekdash.com.tr)

## License

SeekDash is licensed under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`).

Copyright © 2026 Çağan Balkaya.

## Credits and trademark

SeekDash is developed by **Çağan Balkaya** and built with Tauri, Rust, React, and SQLite. Third-party components retain their respective copyrights and license terms.

SeekDash and its logo are project branding. The GPL-3.0-only license applies to the software source; it does not grant trademark rights to the SeekDash name or logo.
