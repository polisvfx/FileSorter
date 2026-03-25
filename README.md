# FileSorter

A cross-platform desktop application that organizes files into subfolders based on user-defined filename rules. Built with Tauri v2 (Rust) and Svelte 5.

## How It Works

FileSorter uses an ordered list of rules to sort files into folders. Each rule has three fields:

| Field | Required | Description |
|---|---|---|
| **Contains** | Yes | Substring to match in filenames (case-insensitive) |
| **Contains NOT** | No | Exclusion substring — skip files that also contain this |
| **Target Folder** | Yes | Folder name to move matching files into |

Rules execute **top-to-bottom in order**. Each rule walks the entire directory tree recursively, so Rule 2 sees the filesystem *after* Rule 1 has already moved files. This lets you build intricate nested folder structures in a single pass.

### Example

Given these rules:

| # | Contains | Contains NOT | Target Folder |
|---|---|---|---|
| 1 | `invoice` | `draft` | `Invoices` |
| 2 | `2024` | | `2024` |

And these files:
```
invoice_2024_001.pdf
invoice_draft_003.pdf
report_2024.pdf
```

After sorting:
```
Invoices/
  2024/
    invoice_2024_001.pdf
2024/
  report_2024.pdf
invoice_draft_003.pdf          ← excluded by "Contains NOT: draft"
```

Rule 1 moves `invoice_2024_001.pdf` into `Invoices/`. Then Rule 2 walks again and finds it inside `Invoices/`, moving it into `Invoices/2024/`.

## Features

- **Drag-and-drop rule reordering** — drag rules to change execution order
- **File & folder input** — drop files/folders from your OS file browser or use the native browse dialog
- **Saveable presets** — save and load rule configurations for different workflows
- **Undo** — reverse the last sort operation, restoring all files to their original locations
- **Filename conflict handling** — appends `(1)`, `(2)`, etc. when a file with the same name already exists in the target folder
- **Cross-platform** — runs natively on Windows, macOS, and Linux

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) stable
- Platform-specific dependencies:
  - **Windows**: Visual Studio Build Tools with C++ workload and Windows SDK
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **macOS**: Xcode Command Line Tools

### Setup

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

The compiled binary will be in `src-tauri/target/release/`.

## Releases

Push a version tag to trigger the GitHub Actions release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds native binaries for Windows (.msi, .exe), macOS (.dmg), and Linux (.deb, .AppImage) and creates a draft GitHub Release.

## License

[GNU General Public License v3.0](LICENSE)
