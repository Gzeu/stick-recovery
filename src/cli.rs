use anyhow::Result;
use crate::device;
use crate::scanner::ScanConfig;
use crate::partition;
use log::info;
use std::path::Path;

pub async fn list_devices() -> Result<()> {
    let devices = device::list_devices().await;

    if devices.is_empty() {
        println!("No removable USB devices found.");
    } else {
        println!("Found {} removable USB device(s):", devices.len());
        println!();
        for device in devices {
            println!("Device: {}", device.path);
            println!("  Model: {}", device.model);
            println!("  Serial: {}", device.serial);
            println!("  Size: {} GB", device.size / (1024 * 1024 * 1024));
            if let Some(mount) = &device.mount_point {
                println!("  Mounted: {}", mount);
            } else {
                println!("  Mounted: No");
            }
            println!();
        }
    }

    Ok(())
}

pub fn show_partitions(device: &str) -> Result<()> {
    println!("Analyzing partitions on: {}", device);
    println!();

    // Check if it's a mounted directory (for testing without root)
    if std::path::Path::new(device).is_dir() {
        println!("Device is a directory (mounted filesystem), listing contents:");
        println!();
        if let Ok(entries) = std::fs::read_dir(device) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    let size_str = if size > 1024 * 1024 {
                        format!("{} MB", size / (1024 * 1024))
                    } else if size > 1024 {
                        format!("{} KB", size / 1024)
                    } else {
                        format!("{} bytes", size)
                    };
                    println!("  {} ({})", entry.file_name().to_string_lossy(), size_str);
                }
            }
        }
        return Ok(());
    }

    let disk_info = partition::analyze_partitions(device)?;

    partition::print_partition_info(&disk_info);

    let fs_partitions = partition::find_filesystem_partitions(&disk_info);
    if !fs_partitions.is_empty() {
        println!("Filesystem partitions found: {}", fs_partitions.len());
        for partition in fs_partitions {
            println!("  Partition {}: {} ({} GB)",
                partition.number,
                partition.partition_type,
                partition.size_bytes / (1024 * 1024 * 1024)
            );
        }
    }

    Ok(())
}

pub async fn scan_device(device: String, scan_type: &str, output: &str) -> Result<()> {
    info!("Starting {} scan on device: {}", scan_type, device);
    info!("Output directory: {}", output);

    // Check if device is mounted
    if device::is_device_mounted(&device) {
        println!("Warning: Device is mounted. For accurate results, unmount first:");
        println!("  sudo umount {}", device);
        println!("Proceeding anyway...");
    }

    let config = ScanConfig {
        scan_type: scan_type.to_string(),
        device: device.clone(),
        output_dir: output.to_string(),
    };

    let recovered = crate::scanner::scan_device(config).await?;

    println!("Scan completed. Found {} recoverable files.", recovered.len());
    println!("Files saved to: {}", output);

    Ok(())
}

pub async fn create_image(device: &str, output: &str) -> Result<()> {
    info!("Creating disk image from device: {}", device);
    info!("Output image: {}", output);

    // Check if device is mounted
    if device::is_device_mounted(device) {
        println!("Warning: Device is mounted. Unmount first:");
        println!("  sudo umount {}", device);
        return Err(anyhow::anyhow!("Device must be unmounted before creating image"));
    }

    println!("Creating disk image...");
    println!("Device: {}", device);
    println!("Output: {}", output);
    println!("This is a placeholder - implement actual disk imaging");

    Ok(())
}

pub async fn recover_from_image(image: &str, output: &str) -> Result<()> {
    info!("Recovering data from image: {}", image);
    info!("Output directory: {}", output);

    let recovered = crate::recovery::recover_files(image, output).await?;

    println!("Recovery completed. Found {} files.", recovered.len());
    println!("Files saved to: {}", output);

    Ok(())
}
