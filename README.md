# stick-recovery

A robust and safe USB stick data recovery tool written in Rust.

[![CI](https://github.com/Gzeu/stick-recovery/actions/workflows/ci.yml/badge.svg)](https://github.com/Gzeu/stick-recovery/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ Safety First

**Read-only by default**: This tool operates in read-only mode unless explicitly
configured otherwise. Writing to the source device requires the `--write` flag
AND interactive confirmation.

**Always work on a disk image first**:
```bash
# Create a forensic image (requires root/administrator)
stick-recovery image --device /dev/sdX --output image.dd --hash sha256

# Recover from the image (safe, repeatable)
stick-recovery recover --input image.dd --output recovered/ --format jpeg,png
```

## Features

- **Disk Imaging**: Sector-by-sector copy with SHA-256 verification
- **File Carving**: Signature-based recovery (JPEG, PNG, PDF, ZIP, MP4, DOCX)
- **Undelete**: FAT32/exFAT directory entry parsing and cluster chain reconstruction
- **NTFS Support**: MFT parsing and $Bitmap analysis
- **CLI**: Clear subcommands with progress reporting and preview

## Requirements

- Root/Administrator privileges for direct device access
- Linux, macOS, or Windows

## Usage

### Create a disk image
```bash
stick-recovery image --device /dev/sdX --output backup.dd
```

### List files on a filesystem
```bash
stick-recovery list --input backup.dd --filesystem fat32
```

### Recover deleted files
```bash
stick-recovery recover --input backup.dd --output ./recovered --undelete
```

### File carving (raw recovery)
```bash
stick-recovery recover --input backup.dd --output ./carved --carve --format jpeg,png,pdf
```

### Verify image integrity
```bash
stick-recovery verify --input backup.dd --hash sha256
```

## Installation

```bash
cargo install stick-recovery
```

Or build from source:
```bash
git clone https://github.com/Gzeu/stick-recovery
cd stick-recovery
cargo build --release
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This tool is provided "as is" without warranty. Always verify recovered data integrity.
Data recovery is not guaranteed; success depends on filesystem state and overwrite patterns.
