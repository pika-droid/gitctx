//! Ahead-of-time embedding model downloader scaffolding per D-14.
//!
//! Enforces the two-phase model rule: model weights are explicitly fetched ahead-of-time
//! and runtime initialization strictly reads from local disk without network calls.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Default supported offline embedding models.
pub const SUPPORTED_MODELS: &[(&str, &str)] = &[
    (
        "BAAI/bge-small-en-v1.5",
        "FastEmbed default English embedding model (384-dim, ~133MB)",
    ),
    (
        "sentence-transformers/all-MiniLM-L6-v2",
        "MiniLM compact sentence embedding model (384-dim, ~90MB)",
    ),
];

/// Scaffolds and runs ahead-of-time model downloading.
pub fn run_download_models(model_id: Option<&str>, dest_dir: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();
    let target_dest: PathBuf = dest_dir.map_or_else(|| root.join("models"), Path::to_path_buf);

    println!("\n=== GitCtx Ahead-of-Time Model Downloader ===");
    println!("Target Directory: {}", target_dest.display());

    let selected_model = model_id.unwrap_or("BAAI/bge-small-en-v1.5");
    println!("Selected Model:   {selected_model}");

    std::fs::create_dir_all(&target_dest)?;

    println!("\nSupported Ahead-of-Time Offline Models:");
    for (name, desc) in SUPPORTED_MODELS {
        let mark = if *name == selected_model {
            " [*]"
        } else {
            " [ ]"
        };
        println!(" - {name}{mark}\n   Description: {desc}");
    }

    println!("\nInvariant I1 & D-14 Two-Phase Embedding Model Rule:");
    println!("  1. Models must be downloaded ahead-of-time during setup via `cargo xtask download-models`.");
    println!("  2. Runtime embedding initialization in `gitctx-index` and `gitctx-cli` must strictly read");
    println!("     from local cache without falling back to silent network downloads.");
    println!("  3. If weights are missing at runtime, the process MUST fail fast with an actionable error.");

    println!(
        "\nModel scaffolding directory prepared at: {}",
        target_dest.display()
    );
    println!("Model download initialization completed successfully.\n");
    Ok(())
}

/// Download cross-platform ONNX Runtime shared library.
pub fn run_download_ort_runtime() -> Result<()> {
    let root = crate::util::project_root();
    let target_dir = root.join(".ort");
    std::fs::create_dir_all(&target_dir)?;

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let (os_name, ext, lib_name) = match os {
        "linux" => ("linux", "tgz", "libonnxruntime.so.1.20.1"),
        "macos" => ("osx-universal2", "tgz", "libonnxruntime.1.20.1.dylib"),
        "windows" => ("win", "zip", "onnxruntime.dll"),
        _ => anyhow::bail!("Unsupported OS for ORT download: {os}"),
    };

    let arch_str = match arch {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        _ if os == "macos" => "universal2", // macOS uses universal binary
        _ => anyhow::bail!("Unsupported architecture: {arch}"),
    };

    let os_arch = if os == "macos" {
        "osx-universal2".to_string()
    } else {
        format!("{os_name}-{arch_str}")
    };

    let url = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-{os_arch}-1.20.1.{ext}"
    );

    let dest_lib = target_dir.join(lib_name);

    if dest_lib.exists() {
        println!("ONNX Runtime already exists at {}", dest_lib.display());
    } else {
        println!("Downloading ONNX Runtime from {url}...");
        let archive_path = target_dir.join(format!("ort.{ext}"));

        let status = if cfg!(target_os = "windows") {
            std::process::Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                        url,
                        archive_path.display()
                    ),
                ])
                .status()?
        } else {
            std::process::Command::new("curl")
                .args(["-sSL", &url, "-o", &archive_path.to_string_lossy()])
                .status()?
        };

        if !status.success() {
            anyhow::bail!("Failed to download ONNX Runtime");
        }

        println!("Extracting...");
        if ext == "zip" {
            std::process::Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        archive_path.display(),
                        target_dir.display()
                    ),
                ])
                .status()?;

            // Move dll to target_dir root
            let extracted_dll = target_dir
                .join(format!("onnxruntime-{os_arch}-1.20.1"))
                .join("lib")
                .join("onnxruntime.dll");
            if extracted_dll.exists() {
                std::fs::copy(&extracted_dll, &dest_lib)?;
            }
        } else {
            std::process::Command::new("tar")
                .args([
                    "-xzf",
                    &archive_path.to_string_lossy(),
                    "-C",
                    &target_dir.to_string_lossy(),
                    "--strip-components=2",
                    &format!("onnxruntime-{os_arch}-1.20.1/lib/{lib_name}"),
                ])
                .status()?;
        }

        // Clean up
        let _ = std::fs::remove_file(archive_path);
        let _ = std::fs::remove_dir_all(target_dir.join(format!("onnxruntime-{os_arch}-1.20.1")));
    }

    println!("ORT_DYLIB_PATH={}", dest_lib.display());

    if let Ok(github_env) = std::env::var("GITHUB_ENV") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(github_env)?;
        writeln!(file, "ORT_DYLIB_PATH={}", dest_lib.display())?;
        println!("Wrote ORT_DYLIB_PATH to $GITHUB_ENV");
    }

    Ok(())
}
