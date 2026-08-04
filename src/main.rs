use clap::{Parser, Subcommand};
use anyhow::Result;

mod device;
mod scanner;
mod recovery;
mod sector;
mod partition;
mod filesystem;
mod cli;

#[derive(Parser)]
#[command(name = "stick-recovery")]
#[command(about = "Advanced cross-platform USB stick data recovery tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connected USB devices
    List,
    /// Show partition information for a device
    Partitions {
        /// Device path (e.g., /dev/sdb or \\\\.\\PhysicalDrive1)
        device: String,
    },
    /// Scan a device for recoverable data
    Scan {
        /// Device path (e.g., /dev/sdb or \\\\.\\PhysicalDrive1)
        device: String,
        /// Scan type: quick, deep, partition, raw
        #[arg(short, long, default_value = "quick")]
        scan_type: String,
        /// Output directory for recovered files
        #[arg(short, long, default_value = "./recovered")]
        output: String,
    },
    /// Create a disk image for offline analysis
    Image {
        /// Device path
        device: String,
        /// Output image file path
        #[arg(short, long)]
        output: String,
    },
    /// Recover data from a disk image
    RecoverImage {
        /// Image file path
        image: String,
        /// Output directory
        #[arg(short, long, default_value = "./recovered")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            cli::list_devices().await?;
        }
        Commands::Partitions { device } => {
            cli::show_partitions(&device)?;
        }
        Commands::Scan { device, scan_type, output } => {
            cli::scan_device(device, &scan_type, &output).await?;
        }
        Commands::Image { device, output } => {
            cli::create_image(&device, &output).await?;
        }
        Commands::RecoverImage { image, output } => {
            cli::recover_from_image(&image, &output).await?;
        }
    }

    Ok(())
}
