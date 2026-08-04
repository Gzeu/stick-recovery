use anyhow::Result;
use crate::sector::SectorReader;

#[derive(Debug, Clone)]
pub enum FileSystemType {
    FAT12,
    FAT16,
    FAT32,
    ExFat,
    NTFS,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FileSystemInfo {
    pub fs_type: FileSystemType,
    pub label: String,
    pub serial: u32,
    pub total_sectors: u64,
    pub free_sectors: u64,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub total_clusters: u32,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub first_cluster: u32,
    pub attributes: u8,
    pub creation_time: Option<u32>,
    pub modification_time: Option<u32>,
    pub access_time: Option<u32>,
}

/// Detect file system type from partition data
pub fn detect_filesystem(partition_data: &[u8]) -> FileSystemType {
    if partition_data.len() < 512 {
        return FileSystemType::Unknown;
    }

    // Check for FAT32 boot sector signature
    if is_fat32_boot_sector(partition_data) {
        return FileSystemType::FAT32;
    }

    // Check for exFAT boot sector
    if is_exfat_boot_sector(partition_data) {
        return FileSystemType::ExFat;
    }

    // Check for NTFS boot sector
    if is_ntfs_boot_sector(partition_data) {
        return FileSystemType::NTFS;
    }

    // Check for FAT16/FAT12
    if is_fat16_boot_sector(partition_data) {
        return FileSystemType::FAT16;
    }

    FileSystemType::Unknown
}

/// Check if boot sector is FAT32
fn is_fat32_boot_sector(data: &[u8]) -> bool {
    if data.len() < 512 {
        return false;
    }

    // Check for boot signature
    if data[510] != 0x55 || data[511] != 0xAA {
        return false;
    }

    // Check for FAT32 signature in boot sector
    let fat32_signature = &data[82..90];
    fat32_signature == b"FAT32   " || fat32_signature == b"FAT32   \0"
}

/// Check if boot sector is exFAT
fn is_exfat_boot_sector(data: &[u8]) -> bool {
    if data.len() < 512 {
        return false;
    }

    // Check for boot signature
    if data[510] != 0x55 || data[511] != 0xAA {
        return false;
    }

    // Check for exFAT signature
    let exfat_signature = &data[3..11];
    exfat_signature == b"EXFAT   "
}

/// Check if boot sector is NTFS
fn is_ntfs_boot_sector(data: &[u8]) -> bool {
    if data.len() < 512 {
        return false;
    }

    // Check for boot signature
    if data[510] != 0x55 || data[511] != 0xAA {
        return false;
    }

    // Check for NTFS signature
    let ntfs_signature = &data[3..11];
    ntfs_signature == b"NTFS    "
}

/// Check if boot sector is FAT16
fn is_fat16_boot_sector(data: &[u8]) -> bool {
    if data.len() < 512 {
        return false;
    }

    // Check for boot signature
    if data[510] != 0x55 || data[511] != 0xAA {
        return false;
    }

    // Check for FAT16 signature (less reliable, need more analysis)
    // This is a simplified check
    let bytes_per_sector = u16::from_le_bytes([data[11], data[12]]);
    let sectors_per_cluster = data[13];
    let reserved_sectors = u16::from_le_bytes([data[14], data[15]]);
    let num_fats = data[16];
    let root_entries = u16::from_le_bytes([data[17], data[18]]);
    let total_sectors_small = u16::from_le_bytes([data[19], data[20]]);

    // FAT16 typically has these characteristics
    bytes_per_sector == 512 &&
    sectors_per_cluster > 0 &&
    reserved_sectors > 0 &&
    num_fats == 2 &&
    root_entries == 512 &&
    total_sectors_small > 0
}

/// Parse FAT32 boot sector
pub fn parse_fat32_boot_sector(data: &[u8]) -> Result<FileSystemInfo> {
    if data.len() < 512 {
        return Err(anyhow::anyhow!("Invalid boot sector size"));
    }

    let bytes_per_sector = u16::from_le_bytes([data[11], data[12]]);
    let sectors_per_cluster = data[13];
    let reserved_sectors = u16::from_le_bytes([data[14], data[15]]);
    let num_fats = data[16];
    let fat_size_32 = u32::from_le_bytes([data[36], data[37], data[38], data[39]]);
    let total_sectors_32 = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    let _root_cluster = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);

    // Volume serial number (offset 67-70)
    let serial = u32::from_le_bytes([data[67], data[68], data[69], data[70]]);

    // Volume label (offset 71-81, 11 bytes)
    let label = String::from_utf8_lossy(&data[71..71 + 11]).trim().to_string();

    let total_sectors = if total_sectors_32 == 0 {
        // Fallback to small sectors count
        u16::from_le_bytes([data[19], data[20]]) as u64
    } else {
        total_sectors_32 as u64
    };

    let data_sectors = total_sectors - reserved_sectors as u64 - (num_fats as u64 * fat_size_32 as u64);
    let _clusters_per_fat = fat_size_32 as u64 * bytes_per_sector as u64 / 4;
    let total_clusters = data_sectors / sectors_per_cluster as u64;

    Ok(FileSystemInfo {
        fs_type: FileSystemType::FAT32,
        label,
        serial,
        total_sectors,
        free_sectors: 0, // Would need to scan FAT table
        bytes_per_sector,
        sectors_per_cluster,
        total_clusters: total_clusters as u32,
    })
}

/// List files in a FAT32 directory
pub fn list_fat32_directory(
    reader: &mut SectorReader,
    cluster: u32,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let bytes_per_cluster = bytes_per_sector as u64 * sectors_per_cluster as u64;

    // Read first cluster
    let cluster_start = cluster as u64 * bytes_per_cluster;
    let cluster_data = reader.read_at(cluster_start, bytes_per_cluster as usize)?;

    // Parse directory entries (32 bytes each)
    for i in 0..(cluster_data.len() / 32) {
        let entry_start = i * 32;
        let entry = &cluster_data[entry_start..entry_start + 32];

        // Check for deleted or empty entries
        if entry[0] == 0x00 {
            break; // End of directory
        }
        if entry[0] == 0xE5 {
            continue; // Deleted entry
        }

        let attributes = entry[11];

        // Skip long file name entries and volume labels
        if attributes == 0x0F || attributes == 0x08 {
            continue;
        }

        let name = parse_fat32_filename(entry);
        let is_directory = (attributes & 0x10) != 0;
        let size = u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]) as u64;
        let first_cluster = u16::from_le_bytes([entry[26], entry[27]]) as u32 |
                        ((entry[20] as u32) << 16);

        entries.push(FileEntry {
            name,
            is_directory,
            size,
            first_cluster,
            attributes,
            creation_time: None,
            modification_time: None,
            access_time: None,
        });
    }

    Ok(entries)
}

/// Parse FAT32 filename from directory entry
fn parse_fat32_filename(entry: &[u8]) -> String {
    // Short filename (8.3 format)
    let main_name = &entry[0..8];
    let extension = &entry[8..11];

    let main_str = String::from_utf8_lossy(main_name).trim().to_string();
    let ext_str = String::from_utf8_lossy(extension).trim().to_string();

    // Note: Long filename parsing would require handling LFN entries
    // This is simplified to short names only
    if !ext_str.is_empty() {
        format!("{}.{}", main_str, ext_str)
    } else {
        main_str
    }
}

/// Read FAT table to find cluster chain
pub fn read_fat_chain(
    reader: &mut SectorReader,
    fat_start_sector: u64,
    bytes_per_sector: u16,
    start_cluster: u32,
) -> Result<Vec<u32>> {
    let mut chain = Vec::new();
    let mut current_cluster = start_cluster;

    loop {
        // FAT32 uses 4 bytes per entry
        let fat_offset = current_cluster as u64 * 4;
        let sector_offset = fat_start_sector + (fat_offset / bytes_per_sector as u64);
        let byte_offset = (fat_offset % bytes_per_sector as u64) as usize;

        let sector_data = reader.read_sector(sector_offset)?;

        if byte_offset + 4 <= sector_data.len() {
            let next_cluster = u32::from_le_bytes([
                sector_data[byte_offset],
                sector_data[byte_offset + 1],
                sector_data[byte_offset + 2],
                sector_data[byte_offset + 3],
            ]);

            // Check for end of chain markers
            if next_cluster >= 0x0FFFFFF8 {
                break;
            }

            chain.push(current_cluster);
            current_cluster = next_cluster;

            // Prevent infinite loops
            if chain.len() > 1000000 {
                break;
            }
        } else {
            break;
        }
    }

    Ok(chain)
}

/// Extract file data from FAT32
pub fn extract_fat32_file(
    reader: &mut SectorReader,
    fat_start_sector: u64,
    data_start_sector: u64,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    first_cluster: u32,
    file_size: u64,
) -> Result<Vec<u8>> {
    let cluster_chain = read_fat_chain(reader, fat_start_sector, bytes_per_sector, first_cluster)?;
    let bytes_per_cluster = bytes_per_sector as u64 * sectors_per_cluster as u64;

    let mut file_data = Vec::new();
    let mut bytes_remaining = file_size;

    for cluster in cluster_chain {
        if bytes_remaining == 0 {
            break;
        }

        let cluster_offset = data_start_sector + (cluster as u64 - 2) * sectors_per_cluster as u64;
        let bytes_to_read = std::cmp::min(bytes_per_cluster, bytes_remaining) as usize;

        let cluster_data = reader.read_at(cluster_offset * bytes_per_sector as u64, bytes_to_read)?;
        file_data.extend_from_slice(&cluster_data);

        bytes_remaining -= bytes_to_read as u64;
    }

    Ok(file_data)
}
