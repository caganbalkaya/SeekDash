# SeekDash privacy information

Last updated: 2026-10-11

SeekDash is a local-first Windows launcher. Its settings, favorites, indexed file metadata, usage history, and optional clipboard history are stored on the user's device.

## Data handled locally

- The search index stores file and application metadata needed to provide results. It does not index file contents.
- Clipboard History is disabled by default and supports plain text only.
- Clipboard persistence is a separate opt-in setting. When disabled, clipboard entries are not written to the database.
- Users can pause clipboard monitoring, delete individual entries, or clear all clipboard history.
- SeekDash does not claim to detect passwords or other sensitive text reliably.
- The application contains no product telemetry or advertising SDK.

## External connections

SeekDash opens external websites or web searches only after a user action. Those requests are then governed by the destination service's privacy policy. If WebView2 is missing, the installer may download Microsoft's supported WebView2 bootstrapper.

## Local data and removal

Uninstalling the application preserves user data by default so that updates and reinstalls do not destroy settings, favorites, indexes, or clipboard history. Local application data can be removed manually only after the user has decided it is no longer needed.

Before sharing diagnostics, screenshots, or logs, remove personal paths, search terms, and clipboard content.

## Contact

Project information and contact options are available at [seekdash.com.tr](https://seekdash.com.tr).
