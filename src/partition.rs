use anyhow::Result;
use crate::sector::{SectorReader, PartitionEntry, partition_type_to_string};

#[derive(Debug, Clone)]
pub enum PartitionScheme {
    MBR,
    GPT,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Partition {
    pub number: u8,
    pub scheme: PartitionScheme,
    pub partition_type: String,
    pub start_sector: u64,
    pub end_sector: u64,
    pub size_bytes: u64,
    pub bootable: bool,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub scheme: PartitionScheme,
    pub partitions: Vec<Partition>,
    pub disk_size: u64,
    pub sector_size: u64,
}

/// Analyze a device to determine partition scheme and extract partition information
pub fn analyze_partitions(device_path: &str) -> Result<DiskInfo> {
    let mut reader = SectorReader::open(device_path)?;
    let disk_size = reader.device_size();
    let sector_size = reader.sector_size();

    // Read MBR (first sector)
    let mbr = reader.read_sector(0)?;

    // Check if we have a valid MBR
    if has_valid_mbr(&mbr) {
        // Check for protective MBR (indicates GPT)
        let partition_table = extract_partition_table(&mbr);
        if is_protective_mbr(&partition_table) {
            return analyze_gpt(&mut reader, disk_size, sector_size);
        } else {
            return analyze_mbr(partition_table, disk_size, sector_size);
        }
    }

    // If no valid MBR, try to detect GPT directly
    if detect_gpt(&mut reader)? {
        return analyze_gpt(&mut reader, disk_size, sector_size);
    }

    // Unknown partition scheme
    Ok(DiskInfo {
        scheme: PartitionScheme::Unknown,
        partitions: Vec::new(),
        disk_size,
        sector_size,
    })
}

/// Check if a device has a valid MBR signature
fn has_valid_mbr(mbr: &[u8]) -> bool {
    if mbr.len() < 512 {
        return false;
    }

    // MBR signature is 0x55 0xAA at offset 510-511
    mbr[510] == 0x55 && mbr[511] == 0xAA
}

/// Extract partition table from MBR
fn extract_partition_table(mbr: &[u8]) -> Vec<PartitionEntry> {
    let mut partitions = Vec::new();

    // Partition table starts at offset 446, 4 entries of 16 bytes each
    for i in 0..4 {
        let offset = 446 + (i * 16);
        if offset + 16 <= mbr.len() {
            let entry = PartitionEntry::from_bytes(&mbr[offset..offset + 16]);
            if entry.is_valid() {
                partitions.push(entry);
            }
        }
    }

    partitions
}

/// Check if MBR is protective (indicates GPT)
fn is_protective_mbr(partitions: &[PartitionEntry]) -> bool {
    partitions.iter().any(|p| p.partition_type == 0xee)
}

/// Analyze MBR partition scheme
fn analyze_mbr(partitions: Vec<PartitionEntry>, disk_size: u64, sector_size: u64) -> Result<DiskInfo> {
    let mut result_partitions = Vec::new();

    for (i, entry) in partitions.iter().enumerate() {
        let partition = Partition {
            number: (i + 1) as u8,
            scheme: PartitionScheme::MBR,
            partition_type: partition_type_to_string(entry.partition_type).to_string(),
            start_sector: entry.start_sector(),
            end_sector: entry.end_sector(),
            size_bytes: entry.size_bytes(),
            bootable: entry.boot_indicator == 0x80,
            active: entry.boot_indicator == 0x80,
        };
        result_partitions.push(partition);
    }

    Ok(DiskInfo {
        scheme: PartitionScheme::MBR,
        partitions: result_partitions,
        disk_size,
        sector_size,
    })
}

/// Detect if disk uses GPT by checking sector 1
fn detect_gpt(reader: &mut SectorReader) -> Result<bool> {
    // GPT Header is at LBA 1 (second sector)
    let gpt_header = reader.read_sector(1)?;

    // Check for GPT signature "EFI PART"
    if gpt_header.len() >= 8 {
        let signature = &gpt_header[0..8];
        return Ok(signature == b"EFI PART");
    }

    Ok(false)
}

/// Analyze GPT partition scheme
fn analyze_gpt(reader: &mut SectorReader, disk_size: u64, sector_size: u64) -> Result<DiskInfo> {
    let mut result_partitions = Vec::new();

    // Read GPT Header
    let gpt_header = reader.read_sector(1)?;

    if gpt_header.len() < 92 {
        return Ok(DiskInfo {
            scheme: PartitionScheme::GPT,
            partitions: Vec::new(),
            disk_size,
            sector_size,
        });
    }

    // Parse GPT Header
    let partition_entry_lba = u64::from_le_bytes([
        gpt_header[72], gpt_header[73], gpt_header[74], gpt_header[75],
        gpt_header[76], gpt_header[77], gpt_header[78], gpt_header[79],
    ]);

    let number_of_entries = u32::from_le_bytes([
        gpt_header[80], gpt_header[81], gpt_header[82], gpt_header[83],
    ]);

    let entry_size = u32::from_le_bytes([
        gpt_header[84], gpt_header[85], gpt_header[86], gpt_header[87],
    ]);

    // Read partition entries
    let entry_start = partition_entry_lba as usize;
    let entry_count = number_of_entries as usize;
    let entry_bytes = entry_size as usize;

    // Limit the number of entries to read to avoid excessive time
    let max_entries = std::cmp::min(entry_count, 128);

    for i in 0..max_entries {
        let entry_sector = entry_start + (i * entry_bytes / sector_size as usize);
        let entry_offset = (i * entry_bytes) % sector_size as usize;

        let sector_data = reader.read_sector(entry_sector as u64)?;

        if entry_offset + entry_bytes <= sector_data.len() {
            let entry = &sector_data[entry_offset..entry_offset + entry_bytes];
            if let Some(partition) = parse_gpt_partition_entry(entry, i as u8 + 1) {
                result_partitions.push(partition);
            }
        }
    }

    Ok(DiskInfo {
        scheme: PartitionScheme::GPT,
        partitions: result_partitions,
        disk_size,
        sector_size,
    })
}

/// Parse a single GPT partition entry
fn parse_gpt_partition_entry(entry: &[u8], number: u8) -> Option<Partition> {
    if entry.len() < 128 {
        return None;
    }

    // Check if partition type is non-zero (empty partitions have type all zeros)
    let partition_type_guid = &entry[0..16];
    if partition_type_guid.iter().all(|&b| b == 0) {
        return None;
    }

    let start_lba = u64::from_le_bytes([
        entry[32], entry[33], entry[34], entry[35],
        entry[36], entry[37], entry[38], entry[39],
    ]);

    let end_lba = u64::from_le_bytes([
        entry[40], entry[41], entry[42], entry[43],
        entry[44], entry[45], entry[46], entry[47],
    ]);

    let attributes = u64::from_le_bytes([
        entry[48], entry[49], entry[50], entry[51],
        entry[52], entry[53], entry[54], entry[55],
    ]);

    let _partition_name = parse_gpt_partition_name(&entry[56..128]);

    Some(Partition {
        number,
        scheme: PartitionScheme::GPT,
        partition_type: guid_to_partition_type(partition_type_guid).to_string(),
        start_sector: start_lba,
        end_sector: end_lba,
        size_bytes: (end_lba - start_lba + 1) * 512,
        bootable: (attributes & 0x4) != 0, // Bit 2 indicates bootable
        active: (attributes & 0x4) != 0,
    })
}

/// Parse UTF-16 partition name from GPT entry
fn parse_gpt_partition_name(name_bytes: &[u8]) -> String {
    let name_utf16: Vec<u16> = name_bytes
        .chunks(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .take_while(|&c| c != 0)
        .collect();

    String::from_utf16_lossy(&name_utf16)
}

/// Convert GUID to human-readable partition type
fn guid_to_partition_type(guid: &[u8]) -> &'static str {
    if guid.len() < 16 {
        return "Unknown";
    }

    // Common GPT partition type GUIDs
    match guid {
        // EFI System Partition
        b"\x28\x73\x2A\xC1\x1F\xF8\xD2\x11\xBA\x4B\x00\xA0\xC9\x3E\xC9\x3B" => "EFI System",
        // Microsoft Reserved
        b"\xE3\xC9\xE3\x16\x0E\x75\xC8\x44\xBC\xDE\x1F\x69\x66\x45\x78\x20" => "Microsoft Reserved",
        // Basic Data (NTFS, exFAT, etc.)
        b"\xA2\xA0\xD0\xEB\xE5\xB9\x33\x44\x87\xC0\x68\xB6\xB7\x26\x99\xC7" => "Basic Data",
        // Linux Filesystem
        b"\x0F\xC6\x3D\xAF\x84\xE3\x47\x9E\xA6\x1C\xF1\x0E\x08\x5E\xC7\xFF" => "Linux Filesystem",
        // Linux Swap
        b"\x06\x57\xFD\x69\x81\x24\x41\xBA\xBB\x95\x05\xF8\x4B\xE3\x2B\x4F" => "Linux Swap",
        // BIOS Boot
        b"\x21\x6F\x5D\xB8\x63\xD9\x49\x4A\x87\x3C\xB5\x61\x6F\xE7\x2C\x6E" => "BIOS Boot",
        // Microsoft Recovery
        b"\xDE\x94\xBA\xA4\x06\xD1\x41\xD4\xA6\x91\x00\x26\x95\xC6\x63\xD3" => "Microsoft Recovery",
        _ => "Unknown GUID",
    }
}

/// Print partition information
pub fn print_partition_info(disk_info: &DiskInfo) {
    println!("Partition Scheme: {:?}", disk_info.scheme);
    println!("Disk Size: {} GB", disk_info.disk_size / (1024 * 1024 * 1024));
    println!("Sector Size: {} bytes", disk_info.sector_size);
    println!("Partitions: {}", disk_info.partitions.len());
    println!();

    for partition in &disk_info.partitions {
        println!("Partition {}: {}", partition.number, partition.partition_type);
        println!("  Start Sector: {}", partition.start_sector);
        println!("  End Sector: {}", partition.end_sector);
        println!("  Size: {} GB", partition.size_bytes / (1024 * 1024 * 1024));
        println!("  Bootable: {}", partition.bootable);
        println!();
    }
}

/// Find all partitions that might contain file systems
pub fn find_filesystem_partitions(disk_info: &DiskInfo) -> Vec<&Partition> {
    disk_info.partitions.iter()
        .filter(|p| {
            matches!(p.partition_type.as_str(),
                "Basic Data" | "Linux Filesystem" | "FAT32" | "FAT16" |
                "NTFS/exFAT" | "FAT12" | "FAT16B" | "FAT16 LBA" | "FAT32 LBA"
            )
        })
        .collect()
}