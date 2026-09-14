//! HyperUser Ring-3 CLI — User-mode interface for the HyperUser hypervisor.
//!
//! Communicates with the hypervisor via overloaded CPUID hypercalls
//! directly from user-mode, without any kernel driver.

mod comm;
mod memory;

use clap::{Parser, Subcommand};
use hyperuser_proto::{CommandId, HypercallPacket, StatusCode};

#[derive(Parser)]
#[command(name = "hyperuser-cli")]
#[command(about = "HyperUser Ring-3 Hypervisor Interface")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Ping the hypervisor to check if it's running
    Ping,
    /// Read physical memory at the given address
    Read {
        /// Physical address to read from (hex, e.g. 0x1000)
        #[arg(value_parser = parse_hex_u64)]
        address: u64,
        /// Number of bytes to read (max 256)
        #[arg(default_value = "64")]
        size: u64,
    },
    /// Write data to physical memory
    Write {
        /// Physical address to write to (hex)
        #[arg(value_parser = parse_hex_u64)]
        address: u64,
        /// Hex string of bytes to write (e.g. "90909090")
        data: String,
    },
    /// Query hypervisor status
    Status,
    /// Enter interactive REPL mode
    Repl,
}

/// Parse a hex string (with optional 0x prefix) into u64
fn parse_hex_u64(s: &str) -> Result<u64, String> {
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u64::from_str_radix(s, 16).map_err(|e| format!("Invalid hex address: {e}"))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Ping) => cmd_ping(),
        Some(Commands::Read { address, size }) => cmd_read(address, size),
        Some(Commands::Write { address, data }) => cmd_write(address, &data),
        Some(Commands::Status) => cmd_status(),
        Some(Commands::Repl) => cmd_repl(),
        None => {
            println!("HyperUser CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("Use --help for usage information, or 'repl' for interactive mode.");
        }
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn cmd_ping() {
    println!("[*] Pinging hypervisor...");
    let mut packet = HypercallPacket::new(CommandId::Ping);
    match comm::hypercall(&mut packet) {
        Ok(()) => {
            if packet.status == StatusCode::Success as u64 {
                println!("[+] Hypervisor is alive!");
            } else {
                println!("[-] Hypervisor returned error status: {}", packet.status);
            }
        }
        Err(e) => println!("[-] Hypercall failed: {e}"),
    }
}

fn cmd_read(address: u64, size: u64) {
    let size = size.min(256);
    println!("[*] Reading {size} bytes from PA {address:#x}...");
    let mut packet = HypercallPacket::new(CommandId::ReadPhys);
    packet.target_pa = address;
    packet.size = size;
    match comm::hypercall(&mut packet) {
        Ok(()) => {
            if packet.status == StatusCode::Success as u64 {
                print_hex_dump(address, &packet.payload[..size as usize]);
            } else {
                println!("[-] Read failed with status: {}", packet.status);
            }
        }
        Err(e) => println!("[-] Hypercall failed: {e}"),
    }
}

fn cmd_write(address: u64, data: &str) {
    let bytes = match hex_string_to_bytes(data) {
        Ok(b) => b,
        Err(e) => {
            println!("[-] Invalid hex data: {e}");
            return;
        }
    };
    if bytes.len() > 256 {
        println!("[-] Data too large (max 256 bytes)");
        return;
    }
    println!("[*] Writing {} bytes to PA {address:#x}...", bytes.len());
    let mut packet = HypercallPacket::new(CommandId::WritePhys);
    packet.target_pa = address;
    packet.size = bytes.len() as u64;
    packet.payload[..bytes.len()].copy_from_slice(&bytes);
    match comm::hypercall(&mut packet) {
        Ok(()) => {
            if packet.status == StatusCode::Success as u64 {
                println!("[+] Write successful");
            } else {
                println!("[-] Write failed with status: {}", packet.status);
            }
        }
        Err(e) => println!("[-] Hypercall failed: {e}"),
    }
}

fn cmd_status() {
    println!("[*] Querying hypervisor status...");
    let mut packet = HypercallPacket::new(CommandId::GetStatus);
    match comm::hypercall(&mut packet) {
        Ok(()) => {
            println!("[+] Hypervisor status code: {}", packet.status);
            // TODO: parse payload for detailed status info
        }
        Err(e) => println!("[-] Hypercall failed: {e}"),
    }
}

fn cmd_repl() {
    println!("HyperUser Interactive Shell v{}", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' for commands, 'exit' to quit.\n");

    let stdin = std::io::stdin();
    let mut line = String::new();

    loop {
        use std::io::Write;
        print!("hyperuser> ");
        std::io::stdout().flush().unwrap();

        line.clear();
        if stdin.read_line(&mut line).is_err() || line.trim() == "exit" {
            println!("Goodbye.");
            break;
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "ping" => cmd_ping(),
            "status" => cmd_status(),
            "read" => {
                if parts.len() < 2 {
                    println!("Usage: read <address> [size]");
                    continue;
                }
                let addr = match parse_hex_u64(parts[1]) {
                    Ok(a) => a,
                    Err(e) => {
                        println!("Error: {e}");
                        continue;
                    }
                };
                let size = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(64u64);
                cmd_read(addr, size);
            }
            "write" => {
                if parts.len() < 3 {
                    println!("Usage: write <address> <hex_data>");
                    continue;
                }
                let addr = match parse_hex_u64(parts[1]) {
                    Ok(a) => a,
                    Err(e) => {
                        println!("Error: {e}");
                        continue;
                    }
                };
                cmd_write(addr, parts[2]);
            }
            "help" => {
                println!("Commands:");
                println!("  ping              - Check if hypervisor is running");
                println!("  status            - Query hypervisor status");
                println!("  read <addr> [sz]  - Read physical memory (hex addr)");
                println!("  write <addr> <hex>- Write physical memory");
                println!("  exit              - Quit");
            }
            _ => println!("Unknown command: '{}'. Type 'help' for commands.", parts[0]),
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Print a formatted hex dump with addresses and ASCII sidebar.
fn print_hex_dump(base_addr: u64, data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:016x}  ", base_addr + (i * 16) as u64);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                print!(" ");
            }
            print!("{:02x} ", byte);
        }
        // Pad remaining columns
        for j in chunk.len()..16 {
            if j == 8 {
                print!(" ");
            }
            print!("   ");
        }
        print!(" |");
        for byte in chunk {
            let c = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            print!("{c}");
        }
        println!("|");
    }
}

/// Parse a hex string (e.g. "90909090" or "0xDEADBEEF") into bytes.
fn hex_string_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    if s.len() % 2 != 0 {
        return Err("Hex string must have even length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {i}: {e}"))
        })
        .collect()
}
