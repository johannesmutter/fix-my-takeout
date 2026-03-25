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
- **Album reconstruction** — rebuilds album folders as aliases (shortcuts) to originals, so files aren't duplicated
- **Browse by type** — quick-access folders for photos, videos, screenshots, favourites, hidden, recently deleted, large files
- **HTML catalogue** — virtual-scrolling grid/list browser with thumbnails and search
- **Nested zip support** — automatically extracts Shared Album zips inside main archives
- **Natural sort order** — processes "Part 1, 2, ... 10" in the right order
- **Auto-updates** — checks for new versions on launch via GitHub Releases

## Download

| | Download |
|---|---|
| **Apple Silicon** (M1, M2, M3, M4) | [Fix My Takeout 1.0.3 (arm64)](https://github.com/johannesmutter/fix-my-takeout/releases/download/v1.0.3/Fix.My.Takeout_1.0.3_aarch64.dmg) |
| **Intel** | [Fix My Takeout 1.0.3 (x64)](https://github.com/johannesmutter/fix-my-takeout/releases/download/v1.0.3/Fix.My.Takeout_1.0.3_x64.dmg) |

[All releases & older versions →](https://github.com/johannesmutter/fix-my-takeout/releases)

**Requirements:** macOS 13 (Ventura) or later.

> **Note:** If macOS says the app is from an unidentified developer, right-click the app and choose **Open**, then click **Open** in the dialog.

## How to use

1. Download your archive (see below)
2. Open Fix My Takeout and select the folder containing your zip files
3. Choose where you want your organized library
4. Click **Start** — the app extracts, catalogs, organizes, deduplicates, and creates browsable views
5. When done, open your library in Finder or browse the HTML catalogue

### How to get your photo export

<details>
<summary><strong>From iCloud</strong></summary>

1. Go to [privacy.apple.com](https://privacy.apple.com) and sign in
2. Click **Request a copy of your data**
3. Select **iCloud Photos** (and optionally other data)
4. Choose a file size (the smaller the size, the more zip files you'll get)
5. Click **Complete Request** — Apple will email you when the files are ready (can take days)
6. Download all the zip files into one folder — don't unzip them, Fix My Takeout handles that

</details>

<details>
<summary><strong>From Google Photos</strong></summary>

1. Go to [takeout.google.com](https://takeout.google.com)
2. Click **Deselect all**, then check only **Google Photos**
3. Click **Next step**, choose **.zip** format and your preferred file size
4. Click **Create export** — Google will email you when ready (can take hours to days)
5. Download all the zip files into one folder — don't unzip them

</details>

### Output structure

Your files are organized by date into year/month folders. Albums, favourites, and other views are created as **aliases** (shortcuts) that point back to the original files — so nothing is duplicated and no extra disk space is used.

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
│   ├── Summer Trip/              ← aliases pointing to originals
│   └── Family/
├── images/                       ← all photos (aliases)
├── videos/                       ← all videos (aliases)
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

## Development

Built with Svelte 5, Tauri v2, Rust, SQLite.

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) (stable)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/johannesmutter/fix-my-takeout.git
cd fix-my-takeout
npm install
npm run tauri dev        # development
npm run tauri build      # production (.app + .dmg in src-tauri/target/release/bundle/)
```

### Code signing

To distribute signed builds, set these GitHub Actions secrets:

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` (`base64 -i Certificates.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` export password |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | App-specific password from [appleid.apple.com](https://appleid.apple.com) |
| `APPLE_TEAM_ID` | 10-char team ID from developer.apple.com |
| `TAURI_SIGNING_PRIVATE_KEY` | From `npx tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Signer password |

### Releasing

Use the release wizard to bump versions, commit, tag, and push:

```bash
python3 scripts/release_wizard.py
```

After the GitHub Actions build completes, verify the release:

```bash
python3 scripts/release_verify.py
```

## License

MIT
