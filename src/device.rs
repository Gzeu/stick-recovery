use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

pub struct Device {
    pub path: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub removable: bool,
    pub mount_point: Option<String>,
}

/// List all removable USB devices on Linux
pub async fn list_devices() -> Vec<Device> {
    let mut devices = Vec::new();

    // Read /sys/block to find block devices
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let device_name = entry.file_name();
            let device_name_str = device_name.to_string_lossy();

            // Skip non-removable devices (like sda which is usually internal)
            if !is_removable_device(&device_name_str) {
                continue;
            }

            // Get device path
            let device_path = format!("/dev/{}", device_name_str);

            // Get device size
            let size = get_device_size(&device_name_str).unwrap_or(0);

            // Get device model and serial
            let model = get_device_model(&device_name_str).unwrap_or_else(|_| "Unknown".to_string());
            let serial = get_device_serial(&device_name_str).unwrap_or_else(|_| "Unknown".to_string());

            // Get mount point if any
            let mount_point = get_mount_point(&device_path);

            devices.push(Device {
                path: device_path,
                model,
                serial,
                size,
                removable: true,
                mount_point,
            });
        }
    }

    devices
}

/// Check if a device is removable (USB)
fn is_removable_device(device_name: &str) -> bool {
    // Skip loop devices, ram disks, etc.
    if device_name.starts_with("loop") || device_name.starts_with("ram") {
        return false;
    }

    // Check the removable flag in sysfs
    let removable_path = format!("/sys/block/{}/removable", device_name);
    if let Ok(removable_content) = fs::read_to_string(&removable_path) {
        let removable = removable_content.trim().parse::<u8>().unwrap_or(0);
        return removable == 1;
    }

    // For USB devices, check if they have a USB parent
    let device_path = format!("/sys/block/{}/device", device_name);
    if Path::new(&device_path).exists() {
        // Check if the device has a USB parent by looking for usb in the device path
        if let Ok(parent_path) = fs::read_link(&device_path) {
            let parent_str = parent_path.to_string_lossy();
            return parent_str.contains("usb");
        }
    }

    false
}

/// Get device size in bytes
fn get_device_size(device_name: &str) -> Result<u64> {
    let size_path = format!("/sys/block/{}/size", device_name);
    let size_content = fs::read_to_string(&size_path)
        .context("Failed to read device size")?;

    let sectors = size_content.trim().parse::<u64>()
        .context("Failed to parse device size")?;

    // Each sector is 512 bytes
    Ok(sectors * 512)
}

/// Get device model name
fn get_device_model(device_name: &str) -> Result<String> {
    let model_path = format!("/sys/block/{}/device/model", device_name);
    let model_content = fs::read_to_string(&model_path)
        .context("Failed to read device model")?;

    Ok(model_content.trim().to_string())
}

/// Get device serial number
fn get_device_serial(device_name: &str) -> Result<String> {
    let serial_path = format!("/sys/block/{}/device/serial", device_name);
    let serial_content = fs::read_to_string(&serial_path)
        .context("Failed to read device serial")?;

    Ok(serial_content.trim().to_string())
}

/// Get mount point for a device
fn get_mount_point(device_path: &str) -> Option<String> {
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == device_path {
                return Some(parts[1].to_string());
            }
        }
    }
    None
}

/// Check if a device is mounted
pub fn is_device_mounted(device_path: &str) -> bool {
    get_mount_point(device_path).is_some()
}

/// Unmount a device (requires root privileges)
pub fn unmount_device(device_path: &str) -> Result<()> {
    let mount_point = get_mount_point(device_path)
        .context("Device is not mounted")?;

    // This would require root privileges and proper error handling
    // For now, just return an error indicating manual intervention needed
    Err(anyhow::anyhow!("Manual unmount required: sudo umount {}", mount_point))
}
