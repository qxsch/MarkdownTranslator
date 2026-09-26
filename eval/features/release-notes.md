# Release notes

## Version 3.2.0 (2026-08-14)

### New features

- **Selective sync for subfolders.** You can now exclude single subfolders instead of whole shares.
- A new `--dry-run` flag for `ctsync reset` shows what would be deleted.
- Dark mode follows the system setting.

### Fixed issues

- Fixed a crash when a file name contained an emoji.
- The tray icon no longer flickers during large uploads.
- Renaming a folder while it was syncing created a duplicate. This is fixed.

### Known issues

- On macOS 14.1, notifications may appear twice. Update to 14.2 to fix this.

## Version 3.1.4 (2026-06-02)

- Security update for CVE-2026-1234. Update as soon as possible.
- Improved upload speed by up to 30% on slow connections.

## Version 3.1.0 (2026-04-20)

### Breaking changes

- The configuration file moved from `config.ini` to `config.toml`. The app migrates it automatically on first start.
- Support for Windows 10 version 1809 ended.
