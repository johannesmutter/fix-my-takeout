# Fix My Takeout

**Fix your iCloud and Google Photos export — organize, deduplicate, and restore your photo library.**

Downloaded your photo archive from iCloud or Google Takeout and ended up with a mess of zip files, JSON sidecars, and duplicate folders? Fix My Takeout turns that chaos into a clean, date-organized library you can actually use.

<p align="center">
  <img src="src-tauri/icons/takeout-cleaner.png" width="128" alt="Fix My Takeout" />
</p>

## Features

- **Handles massive exports** — tested with 6TB+ libraries across 100+ zip files
- **iCloud + Google Takeout** — auto-detects source type, handles both formats
- **Crash-safe** — resume exactly where you left off if interrupted
- **Smart date detection** — pulls dates from Apple CSVs, Google JSON sidecars, EXIF, and filenames
- **Live Photo pairing** — keeps HEIC + MOV together
- **RAW+JPEG pairing** — co-locates DNG/CR2 files with their JPEG counterparts
- **Duplicate detection** — uses Apple checksums or content hashing for Google files
- **Album reconstruction** — rebuilds album folders via symlinks
- **Browse by type** — views for photos, videos, screenshots, favourites, hidden, recently deleted, large files
- **HTML catalogue** — virtual-scrolling grid/list browser with thumbnails and search
- **Nested zip support** — automatically extracts Shared Album zips inside main archives
- **Natural sort order** — processes "Part 1, 2, ... 10" in the right order
- **Auto-updates** — checks for new versions on launch via GitHub Releases

## Download

Download the latest `.dmg` from the [Releases](https://github.com/johannesmutter/fix-my-takeout/releases) page.

**Requirements:** macOS 13 (Ventura) or later.

> **Note:** If macOS says the app is from an unidentified developer, right-click the app and choose **Open**, then click **Open** in the dialog.

## How to use

1. Download your archive from [privacy.apple.com](https://privacy.apple.com) (iCloud) or [takeout.google.com](https://takeout.google.com) (Google Photos)
2. Open Fix My Takeout and select the folder containing your zip files
3. Choose where you want your organized library
4. Click **Start** — the app will extract, catalog, organize, deduplicate, and create browsable views
5. When done, open your library in Finder or browse the HTML catalogue

### Output structure

```
My Library/
├── 2019/
│   ├── 01 January/
│   │   ├── IMG_1234.HEIC
│   │   ├── IMG_1234.MOV          ← Live Photo video kept together
│   │   └── ...
│   └── 02 February/
├── 2020/
├── albums/
│   ├── Summer Trip/              ← symlinks to originals
│   └── Family/
├── images/                       ← all photos by symlink
├── videos/                       ← all videos by symlink
├── screenshots/
├── favourites/
├── large-files/
│   ├── over-10MB/
│   ├── over-100MB/
│   └── over-1GB/
├── duplicates/                   ← separated, safe to delete
├── summary.html                  ← stats and overview
└── catalogue.html                ← browsable photo catalogue
```

---

## Developer guide

### Tech stack

- **Frontend:** Svelte 5 + SvelteKit (static adapter) + Vite
- **Backend:** Rust via Tauri v2
- **Database:** SQLite with WAL mode for crash recovery
- **Pipeline:** Extract → Metadata → Catalog → Pair → Organize → Dedup → Symlinks → Report

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) prerequisites

### Setup

```bash
git clone https://github.com/johannesmutter/fix-my-takeout.git
cd fix-my-takeout
npm install
```

### Development

```bash
npm run tauri dev
```

### Production build

```bash
npm run tauri build
```

The `.app` and `.dmg` will be in `src-tauri/target/release/bundle/`.

### Code signing and notarization

To distribute the app to other Macs, you need an Apple Developer account and a Developer ID Application certificate.

#### One-time setup

1. **Apple Developer certificate:**
   - Open Keychain Access → Certificate Assistant → Request a Certificate from a Certificate Authority
   - Go to [developer.apple.com/account](https://developer.apple.com/account) → Certificates → Create a "Developer ID Application" certificate
   - Download and install the `.cer` file
   - Export the certificate + private key as `.p12` from Keychain Access

2. **App-specific password:**
   - Go to [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords
   - Generate a password for "Fix My Takeout notarization"

3. **Tauri updater key pair:**
   ```bash
   npx tauri signer generate --write-keys "src-tauri/fix-my-takeout.key"
   ```
   Update the `pubkey` in `src-tauri/tauri.conf.json` with the generated public key. Keep the private key secret.

#### GitHub Actions secrets

Set these in your repo's Settings → Secrets → Actions:

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` file (`base64 -i Certificates.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12` |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | The app-specific password |
| `APPLE_TEAM_ID` | Your 10-character team ID from developer.apple.com |
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `src-tauri/fix-my-takeout.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password used during `tauri signer generate` |

#### Creating a release

```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the GitHub Actions workflow which builds, signs, notarizes, and creates a draft release with the DMG attached.

### Project structure

```
src/                          # Svelte frontend
  lib/
    components/               # Welcome, Processing, Done screens
    stores/                   # Svelte stores (progress, settings, zip status)
    tauri.js                  # IPC wrappers and event listeners
src-tauri/
  src/
    commands.rs               # Tauri IPC command handlers
    db/                       # SQLite schema, queries, crash recovery
    fs/                       # Safe file moves, collision handling, disk checks
    metadata/                 # Apple CSV, Google JSON, EXIF, album parsers
    pipeline/                 # Extract → Catalog → Organize → Dedup → Symlink → Report
    progress/                 # Throttled event emitter
```

## License

MIT
