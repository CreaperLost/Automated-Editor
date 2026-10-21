//! The NVIDIA GPU pack: CUDA 13 and cuDNN 9 libraries that ONNX Runtime's CUDA provider needs
//! to run Parakeet and the word aligner on an NVIDIA GPU. They are about 2 GB, so they are not
//! in the installer: transcription settings download them once from NVIDIA's own
//! redistributable archives (each checked against NVIDIA's SHA-256), keep only the DLLs, and
//! load them by full path before the first session is made. Without them ONNX Runtime runs on
//! the CPU, as before.
//!
//! NVIDIA's licences allow shipping these runtime libraries with an application; the
//! download is the same thing, done on the user's machine.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

const REDIST: &str = "https://developer.download.nvidia.com/compute";

/// One of NVIDIA's redistributable archives: its path under [`REDIST`], SHA-256 and size.
struct Archive {
    path: &'static str,
    sha256: &'static str,
    bytes: u64,
}

/// CUDA 13.4 (the ONNX Runtime 1.28 CUDA build is a CUDA 13 build) and cuDNN 9.27 for CUDA 13.
const ARCHIVES: &[Archive] = &[
    Archive {
        path:
            "cuda/redist/cuda_cudart/windows-x86_64/cuda_cudart-windows-x86_64-13.4.92-archive.zip",
        sha256: "621a70c4778287b1abf8c27c61b841d797fda2aeff01033374dd19624bcfa249",
        bytes: 3_437_000,
    },
    Archive {
        path: "cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-13.8.0.4-archive.zip",
        sha256: "0974318e9861a61cb9091cf7de8d4c2880e9787fe311cb4bcbd08865663c5f43",
        bytes: 422_000_000,
    },
    Archive {
        path: "cuda/redist/libcufft/windows-x86_64/libcufft-windows-x86_64-12.4.0.43-archive.zip",
        sha256: "69d0ad8dc3a1be66f01a748a8206d0ceaafa56939474663fbe790dc3e91d2009",
        bytes: 160_000_000,
    },
    Archive {
        path: "cudnn/redist/cudnn/windows-x86_64/cudnn-windows-x86_64-9.27.0.42_cuda13-archive.zip",
        sha256: "15595c7b52896262a0e590f02b3b3851c6a2c9046a72047526deaf59a9cbf7d5",
        bytes: 1_330_803_635,
    },
];

/// The DLLs kept from the archives, in load order: each after the ones it imports (checked
/// with `dumpbin /dependents`). NVRTC is left out: nothing imports it, and without it cuDNN
/// uses its precompiled engines.
pub const DLLS: &[&str] = &[
    "cudart64_13.dll",
    "cublasLt64_13.dll",
    "cublas64_13.dll",
    "cufft64_12.dll",
    "cudnn_graph64_9.dll",
    "cudnn_engines_precompiled64_9.dll",
    "cudnn_engines_runtime_compiled64_9.dll",
    "cudnn_engines_tensor_ir64_9.dll",
    "cudnn_heuristic64_9.dll",
    "cudnn_ops64_9.dll",
    "cudnn_cnn64_9.dll",
    "cudnn_adv64_9.dll",
    "cudnn_ext64_9.dll",
    "cudnn64_9.dll",
];

/// Download size of the whole pack, in bytes.
pub fn download_bytes() -> u64 {
    ARCHIVES.iter().map(|a| a.bytes).sum()
}

pub fn default_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
        .join("runtime")
        .join("cuda13-cudnn9")
}

/// Every DLL of the pack is in `dir`.
pub fn present(dir: &Path) -> bool {
    DLLS.iter().all(|name| dir.join(name).is_file())
}

/// The pack matters only where ONNX Runtime asks for CUDA: Windows builds with `parakeet-cuda`.
pub fn wanted() -> bool {
    cfg!(all(windows, feature = "parakeet-cuda"))
}

/// An NVIDIA driver is installed (its CUDA driver library loads).
pub fn nvidia_driver() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::FreeLibrary;
        use windows_sys::Win32::System::LibraryLoader::{
            LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
        };
        let name: Vec<u16> = "nvcuda.dll\0".encode_utf16().collect();
        // SAFETY: a NUL-terminated name; the module is freed again at once.
        unsafe {
            let module = LoadLibraryExW(
                name.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            );
            if module.is_null() {
                return false;
            }
            FreeLibrary(module);
        }
        true
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Downloads the archives into `dir`, keeps their DLLs and deletes the rest, reporting
/// `(bytes_done, bytes_total)`.
pub fn download(
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let downloads = dir.join("downloads");
    let names: Vec<String> = ARCHIVES
        .iter()
        .map(|a| a.path.rsplit('/').next().unwrap_or(a.path).to_string())
        .collect();
    let files: Vec<(&str, &str, Option<&str>)> = ARCHIVES
        .iter()
        .zip(&names)
        .map(|(a, name)| (a.path, name.as_str(), Some(a.sha256)))
        .collect();
    crate::transcript::parakeet::download_files_checked(
        &downloads, REDIST, &files, cancel, progress,
    )?;
    let unpacked = dir.join("unpacked");
    for name in &names {
        let _ = std::fs::remove_dir_all(&unpacked);
        std::fs::create_dir_all(&unpacked).map_err(|e| e.to_string())?;
        unpack_dlls(&downloads.join(name), &unpacked)?;
        for found in find_files(&unpacked, "dll") {
            let file = found
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if DLLS.iter().any(|d| d.eq_ignore_ascii_case(&file)) {
                std::fs::rename(&found, dir.join(&file))
                    .or_else(|_| std::fs::copy(&found, dir.join(&file)).map(|_| ()))
                    .map_err(|e| format!("Could not keep {file}: {e}"))?;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&unpacked);
    if let Some(missing) = DLLS.iter().find(|name| !dir.join(name).is_file()) {
        return Err(format!("The NVIDIA archives did not hold {missing}"));
    }
    let _ = std::fs::remove_dir_all(&downloads);
    Ok(())
}

/// Unpacks only the DLLs of a zip, with the system's own `tar` (bsdtar on Windows).
fn unpack_dlls(archive: &Path, into: &Path) -> Result<(), String> {
    let tar = std::env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join("System32").join("tar.exe"))
        .unwrap_or_else(|| PathBuf::from("tar"));
    let mut command = std::process::Command::new(&tar);
    command
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .arg("--include=*.dll")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .map_err(|e| format!("Could not run {}: {e}", tar.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Unpacking {} failed: {}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Files under `dir` with extension `ext`, at any depth.
fn find_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
            {
                out.push(path);
            }
        }
    }
    out
}

/// What [`load`] last found, kept for the settings view.
static LOADED: parking_lot::Mutex<Option<Result<(), String>>> = parking_lot::Mutex::new(None);

/// Loads the pack's DLLs by full path so ONNX Runtime's CUDA provider finds them when it
/// loads them by name. Once loaded they stay; a failure is tried again next time (the pack
/// may have been downloaded since). An error means sessions run on the CPU; never fatal.
pub fn load() -> Result<(), String> {
    let mut loaded = LOADED.lock();
    if !matches!(*loaded, Some(Ok(()))) {
        *loaded = Some(load_from(&default_dir()));
    }
    loaded.clone().unwrap_or(Ok(()))
}

/// The outcome of the last [`load`], if one ran.
pub fn loaded() -> Option<Result<(), String>> {
    LOADED.lock().clone()
}

fn load_from(dir: &Path) -> Result<(), String> {
    if !wanted() {
        return Err("this build does not use CUDA".into());
    }
    if !nvidia_driver() {
        return Err("no NVIDIA driver is installed".into());
    }
    if !present(dir) {
        return Err("the NVIDIA GPU pack is not downloaded".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::LibraryLoader::{
            LoadLibraryExW, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        };
        for name in DLLS {
            let wide: Vec<u16> = dir
                .join(name)
                .as_os_str()
                .encode_wide()
                .chain([0])
                .collect();
            // SAFETY: a NUL-terminated absolute path. The modules stay loaded for the
            // process's life: ONNX Runtime keeps using them.
            let module = unsafe {
                LoadLibraryExW(
                    wide.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
                )
            };
            if module.is_null() {
                return Err(format!(
                    "{name} would not load ({})",
                    std::io::Error::last_os_error()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_needs_every_dll() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!present(dir.path()));
        for name in DLLS {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        assert!(present(dir.path()));
        std::fs::remove_file(dir.path().join("cudnn64_9.dll")).unwrap();
        assert!(!present(dir.path()));
    }

    /// `AEROEDITS_CUDA_PROBE=<folder of the archives> cargo test --release --lib --features
    /// parakeet-cuda cuda_pack_probe -- --ignored --nocapture`: installs the pack from archives
    /// already downloaded (they count as the download), then loads it.
    #[test]
    #[ignore]
    fn cuda_pack_probe() {
        let from = std::path::PathBuf::from(std::env::var("AEROEDITS_CUDA_PROBE").unwrap());
        let dir = default_dir();
        std::fs::create_dir_all(dir.join("downloads")).unwrap();
        for archive in ARCHIVES {
            let name = archive.path.rsplit('/').next().unwrap();
            if !dir.join("downloads").join(name).is_file() {
                std::fs::copy(from.join(name), dir.join("downloads").join(name)).unwrap();
            }
        }
        let started = std::time::Instant::now();
        download(&dir, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        println!(
            "installed in {:?} into {}",
            started.elapsed(),
            dir.display()
        );
        assert!(present(&dir));
        assert!(!dir.join("downloads").exists());
        println!("nvidia driver: {}, load: {:?}", nvidia_driver(), load());
    }

    #[test]
    fn archives_are_pinned() {
        for archive in ARCHIVES {
            assert_eq!(archive.sha256.len(), 64, "{}", archive.path);
            assert!(archive.path.ends_with("-archive.zip"));
        }
        assert!(download_bytes() > 1_500_000_000);
    }
}
