use anyhow::{Result, Context};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub struct SectorReader {
    file: File,
    device_size: u64,
    sector_size: u64,
    current_sector: u64,
}

impl SectorReader {
    /// Open a device or file for sector reading
    pub fn open(path: &str) -> Result<Self> {
        let file = File::open(path)
            .context("Failed to open device")?;

        let device_size = file.metadata()
            .context("Failed to get device metadata")?
            .len();

        // Standard sector size is 512 bytes
        let sector_size = 512;

        Ok(SectorReader {
            file,
            device_size,
            sector_size,
            current_sector: 0,
        })
    }

    /// Get total device size in bytes
    pub fn device_size(&self) -> u64 {
        self.device_size
    }

    /// Get sector size (usually 512 bytes)
    pub fn sector_size(&self) -> u64 {
        self.sector_size
    }

    /// Get total number of sectors
    pub fn total_sectors(&self) -> u64 {
        self.device_size / self.sector_size
    }

    /// Read a single sector by sector number
    pub fn read_sector(&mut self, sector: u64) -> Result<Vec<u8>> {
        let offset = sector * self.sector_size;
        self.read_at(offset, self.sector_size as usize)
    }

    /// Read multiple sectors
    pub fn read_sectors(&mut self, start_sector: u64, count: u64) -> Result<Vec<u8>> {
        let offset = start_sector * self.sector_size;
        let size = (count * self.sector_size) as usize;
        self.read_at(offset, size)
    }

    /// Read data at a specific offset
    pub fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(offset))
            .context("Failed to seek to offset")?;

        let mut buffer = vec![0u8; size];
        self.file.read_exact(&mut buffer)
            .context("Failed to read data")?;

        Ok(buffer)
    }

    /// Seek to a specific sector
    pub fn seek_to_sector(&mut self, sector: u64) -> Result<()> {
        let offset = sector * self.sector_size;
        self.file.seek(SeekFrom::Start(offset))
            .context("Failed to seek to sector")?;
        self.current_sector = sector;
        Ok(())
    }

    /// Read the next sector (sequential reading)
    pub fn read_next_sector(&mut self) -> Result<Vec<u8>> {
        let sector = self.read_sector(self.current_sector)?;
        self.current_sector += 1;
        Ok(sector)
    }

    /// Get current sector position
    pub fn current_sector(&self) -> u64 {
        self.current_sector
    }

    /// Check if we've reached the end of the device
    pub fn is_end(&self) -> bool {
        self.current_sector >= self.total_sectors()
    }

    /// Read sectors with progress callback
    pub fn read_with_progress<F>(
        &mut self,
        start_sector: u64,
        count: u64,
        progress_callback: F,
    ) -> Result<Vec<u8>>
    where
        F: Fn(u64, u64), // callback(current_sector, total_sectors)
    {
        let offset = start_sector * self.sector_size;
        let size = (count * self.sector_size) as usize;

        self.file.seek(SeekFrom::Start(offset))
            .context("Failed to seek to start sector")?;

        let mut buffer = vec![0u8; size];
        let total_sectors = count;
        let mut sectors_read = 0u64;

        // Read in chunks for better performance
        let chunk_size = 1024 * 1024; // 1MB chunks
        let mut bytes_read = 0usize;

        while bytes_read < size {
            let bytes_to_read = std::cmp::min(chunk_size, size - bytes_read);
            self.file.read_exact(&mut buffer[bytes_read..bytes_read + bytes_to_read])
                .context("Failed to read chunk")?;

            bytes_read += bytes_to_read;
            let sectors_in_chunk = (bytes_to_read / self.sector_size as usize) as u64;
            sectors_read += sectors_in_chunk;

            progress_callback(sectors_read, total_sectors);
        }

        Ok(buffer)
    }
}

/// Read the Master Boot Record (MBR) from a device
pub fn read_mbr(device_path: &str) -> Result<Vec<u8>> {
    let mut reader = SectorReader::open(device_path)?;
    reader.read_sector(0)
}

/// Check if a device has a valid MBR signature
pub fn has_valid_mbr(mbr: &[u8]) -> bool {
    if mbr.len() < 512 {
        return false;
    }

    // MBR signature is 0x55 0xAA at offset 510-511
    mbr[510] == 0x55 && mbr[511] == 0xAA
}

/// Extract partition table from MBR
pub fn extract_partition_table(mbr: &[u8]) -> Vec<PartitionEntry> {
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

#[derive(Debug, Clone)]
pub struct PartitionEntry {
    pub boot_indicator: u8,
    pub starting_chs: [u8; 3],
    pub partition_type: u8,
    pub ending_chs: [u8; 3],
    pub starting_lba: u32,
    pub total_sectors: u32,
}

impl PartitionEntry {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        PartitionEntry {
            boot_indicator: bytes[0],
            starting_chs: [bytes[1], bytes[2], bytes[3]],
            partition_type: bytes[4],
            ending_chs: [bytes[5], bytes[6], bytes[7]],
            starting_lba: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            total_sectors: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        }
    }

    pub fn is_valid(&self) -> bool {
        // Valid partition if partition_type is not 0x00 (empty)
        self.partition_type != 0x00
    }

    pub fn start_sector(&self) -> u64 {
        self.starting_lba as u64
    }

    pub fn end_sector(&self) -> u64 {
        self.starting_lba as u64 + self.total_sectors as u64 - 1
    }

    pub fn size_bytes(&self) -> u64 {
        self.total_sectors as u64 * 512
    }
}

/// Convert partition type to human-readable string
pub fn partition_type_to_string(partition_type: u8) -> &'static str {
    match partition_type {
        0x00 => "Empty",
        0x01 => "FAT12",
        0x04 => "FAT16 < 32M",
        0x05 => "Extended",
        0x06 => "FAT16B",
        0x07 => "NTFS/exFAT",
        0x0b => "FAT32",
        0x0c => "FAT32 LBA",
        0x0e => "FAT16 LBA",
        0x0f => "Extended LBA",
        0x82 => "Linux Swap",
        0x83 => "Linux",
        0x85 => "Linux Extended",
        0xee => "GPT Protective",
        0xef => "EFI System",
        _ => "Unknown",
    }
}