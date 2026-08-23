use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

const ORT_VERSION: &str = "1.22.0";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ORT_DYLIB_PATH");

    if let Err(e) = setup_onnxruntime() {
        println!("cargo:warning=gitctx-index build.rs could not auto-provision ONNX Runtime: {e}");
    }
}

fn setup_onnxruntime() -> Result<(), Box<dyn std::error::Error>> {
    let target = env::var("TARGET").unwrap_or_default();
    let (url, filename, lib_filename) = match target.as_str() {
        t if t.contains("windows") && t.contains("x86_64") => (
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-win-x64-{ORT_VERSION}.zip"),
            format!("onnxruntime-win-x64-{ORT_VERSION}.zip"),
            "onnxruntime.dll",
        ),
        t if t.contains("apple") && (t.contains("aarch64") || t.contains("arm64")) => (
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-arm64-{ORT_VERSION}.tgz"),
            format!("onnxruntime-osx-arm64-{ORT_VERSION}.tgz"),
            "libonnxruntime.dylib",
        ),
        t if t.contains("apple") && t.contains("x86_64") => (
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-x64-{ORT_VERSION}.tgz"),
            format!("onnxruntime-osx-x64-{ORT_VERSION}.tgz"),
            "libonnxruntime.dylib",
        ),
        t if t.contains("linux") && (t.contains("aarch64") || t.contains("arm64")) => (
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-linux-aarch64-{ORT_VERSION}.tgz"),
            format!("onnxruntime-linux-aarch64-{ORT_VERSION}.tgz"),
            "libonnxruntime.so",
        ),
        t if t.contains("linux") && t.contains("x86_64") => (
            format!("https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-linux-x64-{ORT_VERSION}.tgz"),
            format!("onnxruntime-linux-x64-{ORT_VERSION}.tgz"),
            "libonnxruntime.so",
        ),
        _ => return Ok(()),
    };

    let cache_dir = env::temp_dir().join(format!("gitctx-ort-{ORT_VERSION}-{target}"));
    fs::create_dir_all(&cache_dir)?;
    let cached_lib = cache_dir.join(lib_filename);

    if !cached_lib.exists() {
        let archive_path = cache_dir.join(&filename);
        if !archive_path.exists() {
            let resp = ureq::get(&url)
                .timeout(std::time::Duration::from_secs(60))
                .call()?;
            let mut reader = resp.into_reader();
            let mut out = File::create(&archive_path)?;
            std::io::copy(&mut reader, &mut out)?;
        }

        let path_obj = Path::new(&filename);
        let ext = path_obj.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext.eq_ignore_ascii_case("zip") {
            extract_from_zip(&archive_path, lib_filename, &cached_lib)?;
        } else if ext.eq_ignore_ascii_case("tgz") || ext.eq_ignore_ascii_case("gz") {
            extract_from_tgz(&archive_path, lib_filename, &cached_lib)?;
        }
    }

    if cached_lib.exists() {
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_path = PathBuf::from(out_dir);
            if let Some(target_dir) = out_path.ancestors().nth(3) {
                let dest_dirs = [
                    target_dir.to_path_buf(),
                    target_dir.join("deps"),
                    target_dir.join("examples"),
                ];
                for dir in dest_dirs {
                    let _ = fs::create_dir_all(&dir);
                    let dest = dir.join(lib_filename);
                    let _ = fs::copy(&cached_lib, &dest);
                }
            }
        }

        // Also copy to workspace root / current dir for cargo test running from workspace root
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            let root = PathBuf::from(manifest_dir).join("../..");
            let _ = fs::copy(&cached_lib, root.join(lib_filename));
        }

        println!("cargo:rustc-env=ORT_DYLIB_PATH={}", cached_lib.display());
    }

    Ok(())
}

fn extract_from_zip(
    zip_path: &Path,
    target_lib: &str,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(target_lib) && !file.is_dir() {
            let mut out = File::create(output)?;
            std::io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }
    Err(format!("Could not find {target_lib} inside zip archive").into())
}

fn extract_from_tgz(
    tgz_path: &Path,
    target_lib: &str,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(tgz_path)?;
    let gz = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let path_str = path.to_string_lossy();
        if path_str.ends_with(target_lib) {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            fs::write(output, content)?;
            return Ok(());
        }
    }
    Err(format!("Could not find {target_lib} inside tgz archive").into())
}
