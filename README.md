# Stick Recovery

Advanced cross-platform USB stick data recovery tool written in Rust.

## Features

- **Cross-platform**: Windows, Linux, macOS
- **Multiple scan strategies**: Quick Scan, Deep Scan, Partition Recovery, Raw Recovery
- **File type detection**: Documents, Media, Archives, System files
- **Preview and validation**: Preview files before recovery
- **Advanced export**: Preserve directory structure, metadata preservation
- **CLI interface**: Full command-line interface for automation

## Installation

### From Source

```bash
cargo install --path .
```

### Build from Source

```bash
git clone https://github.com/Gzeu/stick-recovery.git
cd stick-recovery
cargo build --release
```

## Usage

### List Devices

```bash
stick-recovery list
```

### Scan Device

```bash
# Quick scan (default)
stick-recovery scan /dev/sdb --output ./recovered

# Deep scan
stick-recovery scan /dev/sdb --scan-type deep --output ./recovered

# Partition recovery
stick-recovery scan /dev/sdb --scan-type partition --output ./recovered

# Raw recovery
stick-recovery scan /dev/sdb --scan-type raw --output ./recovered
```

### Create Disk Image

```bash
stick-recovery image /dev/sdb --output disk_image.dd
```

### Recover from Image

```bash
stick-recovery recover-image disk_image.dd --output ./recovered
```

## Scan Types

### Quick Scan
- Scans file system tables for recently deleted files
- Fast (minutes)
- Best for accidental deletions

### Deep Scan
- Sector-by-sector scan of entire disk
- Slow (hours)
- Recovers corrupted and fragmented files

### Partition Recovery
- Reconstructs MBR/GPT partition tables
- Recovers lost partitions
- Restores partition structure

### Raw Recovery
- Signature-based file detection
- Works when file system is completely destroyed
- Recovers files regardless of original file system

## Supported File Types

### Documents
- Office: DOC, DOCX, XLS, XLSX, PPT, PPTX
- PDF: PDF (including corrupted)
- Text: TXT, RTF, ODT
- Email: PST, OST, MBOX, EML

### Media
- Images: JPG, PNG, GIF, BMP, TIFF, RAW (CR2, NEF, ARW)
- Video: MP4, AVI, MOV, MKV, WMV
- Audio: MP3, WAV, FLAC, AAC, OGG

### Archives
- ZIP, RAR, 7Z, TAR, GZ

### System
- EXE, DLL, SO, DYLIB
- CFG, INI, JSON, XML

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Release Build

```bash
cargo build --release
```

## Architecture

```
├── Core Engine
│   ├── Device Detection
│   ├── Sector Reader
│   ├── Partition Analyzer
│   ├── File System Parser
│   └── Data Carver
│
├── Recovery Strategies
│   ├── Quick Scan
│   ├── Deep Scan
│   ├── Partition Recovery
│   └── Raw Recovery
│
├── File Type Detection
│   ├── Document Signatures
│   ├── Media Signatures
│   ├── Archive Signatures
│   └── System Signatures
│
├── Preview & Validation
│   ├── File Preview
│   ├── Integrity Check
│   └── Metadata Extraction
│
└── Export Options
    ├── Original Structure
    ├── Flat Export
    ├── Filtered Export
    └── Image Creation
```

## Safety Notes

- **Always work on disk images first** when dealing with critically corrupted drives
- **Never write to the source disk** - only read operations
- **Stop if you hear unusual noises** - may indicate physical failure
- **Consider professional recovery** for physically damaged drives

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please read CONTRIBUTING.md for guidelines.

## Status

Currently in development - MVP expected in 10-15 weeks.

## Authors

Gzeu - Initial implementation
