//! GPU detection and VRAM querying utilities.
//!
//! This module provides functions to detect available GPUs,
//! query VRAM information, and determine optimal GPU configurations.

use std::process::Command;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Error types for GPU detection operations.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// Failed to execute a system command for GPU detection.
    #[error("Failed to execute command: {0}")]
    CommandError(String),
    /// Failed to parse the output of a GPU detection command.
    #[error("Failed to parse output: {0}")]
    ParseError(String),
    /// No GPU was detected on the system.
    #[error("No GPU detected")]
    NoGpuDetected,
    /// Vulkan runtime is not available on the system.
    #[error("Vulkan not available")]
    VulkanNotAvailable,
}

/// Check if Vulkan is available on the system.
pub fn vulkan_available() -> bool {
    // Try to run vulkaninfo to check Vulkan availability
    match Command::new("vulkaninfo").output() {
        Ok(output) => output.status.success(),
        Err(_) => {
            // Fallback: check for vulkan library
            std::path::Path::new("/usr/lib/x86_64-linux-gnu/libvulkan.so.1").exists() || std::path::Path::new("/usr/lib/x86_64-linux-gnu/libvulkan.so").exists()
        }
    }
}

/// Check if the system has a discrete GPU.
pub fn has_discrete_gpu() -> bool {
    // Use lspci to detect discrete GPUs
    match Command::new("lspci").arg("-nn").arg("-d").arg("::0300").output() {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Look for discrete GPU indicators
            output_str.contains("VGA") ||
                output_str.contains("3D controller") ||
                output_str.contains("Display") ||
                // Common GPU vendors
                output_str.contains("NVIDIA") ||
                output_str.contains("AMD") ||
                output_str.contains("ATI") ||
                output_str.contains("Intel") && output_str.contains("Xe")
        }
        Err(_) => false,
    }
}

/// Get available VRAM in MB from the system.
pub fn get_available_vram() -> usize {
    // Try multiple methods to get VRAM information

    // Method 1: Use nvidia-smi for NVIDIA GPUs
    if let Ok(vram) = get_nvidia_vram() {
        return vram;
    }

    // Method 2: Use AMD GPU tools
    if let Ok(vram) = get_amd_vram() {
        return vram;
    }

    // Method 3: Use Vulkan info
    if let Ok(vram) = get_vulkan_vram() {
        return vram;
    }

    // Method 4: Use lspci as fallback
    get_lspci_vram().unwrap_or(4096) // Default to 4GB
}

/// Get VRAM from NVIDIA GPU using nvidia-smi.
fn get_nvidia_vram() -> Result<usize, GpuError> {
    let output = Command::new("nvidia-smi")
        .args(&["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .map_err(|e| GpuError::CommandError(e.to_string()))?;

    if !output.status.success() {
        return Err(GpuError::CommandError("nvidia-smi failed".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let vram_mb = output_str
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<usize>().ok())
        .ok_or_else(|| GpuError::ParseError("Failed to parse nvidia-smi output".to_string()))?;

    debug!("NVIDIA VRAM detected: {} MB", vram_mb);
    Ok(vram_mb)
}

/// Get VRAM from AMD GPU using radeontop or rocm-smi.
fn get_amd_vram() -> Result<usize, GpuError> {
    // Try rocm-smi first
    if let Ok(output) = Command::new("rocm-smi").arg("--showmeminfo").arg("vram").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse VRAM from rocm-smi output
        for line in output_str.lines() {
            if line.contains("VRAM Total") {
                if let Some(vram_str) = line.split(':').nth(1) {
                    if let Ok(vram_mb) = vram_str.trim().replace("MB", "").parse::<usize>() {
                        debug!("AMD ROCm VRAM detected: {} MB", vram_mb);
                        return Ok(vram_mb);
                    }
                }
            }
        }
    }

    // Try radeontop as fallback
    if let Ok(output) = Command::new("radeontop").args(&["-b", "-l", "1"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse VRAM from radeontop output
        for line in output_str.lines() {
            if line.contains("VRAM") && line.contains("MB") {
                if let Some(vram_str) = line.split_whitespace().find(|s| s.ends_with("MB")) {
                    if let Ok(vram_mb) = vram_str.replace("MB", "").parse::<usize>() {
                        debug!("AMD radeontop VRAM detected: {} MB", vram_mb);
                        return Ok(vram_mb);
                    }
                }
            }
        }
    }

    Err(GpuError::CommandError("AMD GPU tools not available".to_string()))
}

/// Get VRAM from Vulkan info.
fn get_vulkan_vram() -> Result<usize, GpuError> {
    let output = Command::new("vulkaninfo").output().map_err(|e| GpuError::CommandError(e.to_string()))?;

    if !output.status.success() {
        return Err(GpuError::CommandError("vulkaninfo failed".to_string()));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Parse VRAM from Vulkan info
    for line in output_str.lines() {
        if line.contains("deviceType") && line.contains("DISCRETE_GPU") {
            // Look for VRAM in subsequent lines
            continue;
        }

        if line.contains("maxMemoryAllocationCount") && line.contains("0x") {
            // Extract memory size from hex value (this is approximate)
            if let Some(hex_str) = line.split('=').nth(1) {
                if let Ok(memory_size) = usize::from_str_radix(hex_str.trim().trim_start_matches("0x"), 16) {
                    let vram_mb = memory_size / (1024 * 1024); // Convert bytes to MB
                    if vram_mb > 100 && vram_mb < 128000 {
                        // Reasonable range: 100MB - 128GB
                        debug!("Vulkan VRAM detected: {} MB", vram_mb);
                        return Ok(vram_mb);
                    }
                }
            }
        }
    }

    Err(GpuError::ParseError("Failed to parse Vulkan VRAM info".to_string()))
}

/// Get VRAM from lspci as fallback.
fn get_lspci_vram() -> Result<usize, GpuError> {
    let output = Command::new("lspci")
        .args(&["-v", "-nn", "-d", "::0300"])
        .output()
        .map_err(|e| GpuError::CommandError(e.to_string()))?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Parse VRAM from lspci output (less reliable)
    for line in output_str.lines() {
        if line.contains("Size=") {
            if let Some(size_str) = line.split("Size=").nth(1) {
                if let Some(size_mb) = size_str.split('M').next() {
                    if let Ok(vram_mb) = size_mb.parse::<usize>() {
                        debug!("lspci VRAM detected: {} MB", vram_mb);
                        return Ok(vram_mb);
                    }
                }
            }
        }
    }

    Err(GpuError::ParseError("Failed to parse lspci VRAM info".to_string()))
}

/// Get total system RAM in MB.
pub fn get_system_ram_mb() -> usize {
    if let Ok(output) = Command::new("free").arg("-m").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if line.starts_with("Mem:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(total_mb) = parts[1].parse::<usize>() {
                        return total_mb;
                    }
                }
            }
        }
    }

    // Fallback: assume 8GB
    warn!("Could not detect system RAM, assuming 8GB");
    8192
}

/// Get system memory information (alias for get_system_ram_mb).
pub fn get_system_memory() -> usize {
    get_system_ram_mb()
}

/// Check if ROCm/HIPBLAS libraries are actually available on the system.
pub fn hipblas_libraries_available() -> bool {
    use glob::glob;

    // Check for ROCm/HIPBLAS library presence in common locations
    std::path::Path::new("/opt/rocm/lib/libhipblas.so").exists() ||
        std::path::Path::new("/usr/lib/x86_64-linux-gnu/libhipblas.so").exists() ||
        std::path::Path::new("/usr/local/rocm/lib/libhipblas.so").exists() ||
        // Ollama-specific ROCm installation paths (v7.2)
        std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblas.so").exists() ||
        std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblas.so.3").exists() ||
        std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblaslt.so").exists() ||
        std::path::Path::new("/usr/local/lib/ollama/rocm_v7_2/libhipblaslt.so.1").exists() ||
        // Generic Ollama ROCm path pattern for any version
        if let Ok(mut paths) = glob("/usr/local/lib/ollama/rocm_v*/libhipblas.so*") {
            paths.next().is_some()
        } else {
            false
        }
}

/// Check if the discrete GPU is AMD (for ROCm optimization).
pub fn is_amd_discrete_gpu() -> bool {
    // This would query GPU vendor information
    // For now, assume AMD if we have a discrete GPU on Linux
    cfg!(target_os = "linux") && has_discrete_gpu()
}

/// Query detailed VRAM information.
pub fn query_vram_info() -> Result<usize, GpuError> {
    let vram = get_available_vram();

    if vram == 0 {
        return Err(GpuError::NoGpuDetected);
    }

    info!("Detected VRAM: {} MB", vram);
    Ok(vram)
}

/// Get GPU information including vendor and model.
pub fn get_gpu_info() -> Result<GpuInfo, GpuError> {
    let output = Command::new("lspci")
        .args(&["-nn", "-d", "::0300"])
        .output()
        .map_err(|e| GpuError::CommandError(e.to_string()))?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if line.contains("VGA") || line.contains("3D controller") || line.contains("Display") {
            let vendor = if line.contains("NVIDIA") {
                "NVIDIA"
            } else if line.contains("AMD") || line.contains("ATI") {
                "AMD"
            } else if line.contains("Intel") {
                "Intel"
            } else {
                "Unknown"
            };

            let model = line.split(':').last().unwrap_or("Unknown").trim();

            return Ok(GpuInfo {
                vendor: vendor.to_string(),
                model: model.to_string(),
                is_discrete: !line.contains("Intel") || line.contains("Xe"),
                vram_mb: get_available_vram(),
            });
        }
    }

    Err(GpuError::NoGpuDetected)
}

/// GPU information structure.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// GPU vendor name (e.g. "NVIDIA", "AMD", "Intel").
    pub vendor: String,
    /// GPU model name from lspci.
    pub model: String,
    /// Whether the GPU is discrete (dedicated VRAM) or integrated.
    pub is_discrete: bool,
    /// Available VRAM in megabytes.
    pub vram_mb: usize,
}

impl std::fmt::Display for GpuInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({}VRAM: {} MB)",
            self.vendor,
            self.model,
            if self.is_discrete { "Discrete, " } else { "Integrated, " },
            self.vram_mb
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_available() {
        // This test will pass if Vulkan is available
        let available = vulkan_available();
        println!("Vulkan available: {}", available);
    }

    #[test]
    fn test_gpu_detection() {
        let has_gpu = has_discrete_gpu();
        println!("Has discrete GPU: {}", has_gpu);

        if let Ok(gpu_info) = get_gpu_info() {
            println!("GPU Info: {}", gpu_info);
        }
    }

    #[test]
    fn test_vram_detection() {
        let vram = get_available_vram();
        println!("Available VRAM: {} MB", vram);

        let system_ram = get_system_ram_mb();
        println!("System RAM: {} MB", system_ram);
    }
}
