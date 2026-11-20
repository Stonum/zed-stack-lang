use std::{fs, io, path::Path};

use sha2::{Digest, Sha256};
use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, LanguageServerId,
    LanguageServerInstallationStatus as Status, Os, Result,
};

// Development pin. Update the tag and digests together after server acceptance.
const RELEASE_TAG: &str = "v0.8.3";
const RELEASES: &str = "https://github.com/Stonum/stack-lang-server/releases/download";
const LOCAL_CONFIGURATION: &str = "Set lsp.stack-lang-server.binary.path (or lsp.stack-sql-server.binary.path for SQL) or add stack-lang-server to PATH.";

struct Asset {
    name: &'static str,
    sha256: &'static str,
    needs_executable_permission: bool,
}

fn asset(os: Os, arch: Architecture) -> Option<Asset> {
    let (name, sha256, needs_executable_permission) = match (os, arch) {
        (Os::Windows, Architecture::X8664) => (
            "stack-lang-server-win32-x64.exe",
            "6d01ca9b83236f2284d13988f2abb349d3e66bc9a290276d07c51782cf8e6633",
            false,
        ),
        (Os::Linux, Architecture::X8664) => (
            "stack-lang-server-linux-x64",
            "42579da44433d5cfe46461ceaec566e2710de8485c1e5b12554b8c4f1a20a296",
            true,
        ),
        _ => return None,
    };
    Some(Asset {
        name,
        sha256,
        needs_executable_permission,
    })
}

pub fn install(id: &LanguageServerId) -> Result<String> {
    let (os, arch) = zed::current_platform();
    let result = asset(os, arch)
        .ok_or_else(|| {
            format!("No Stack language server download for this platform. {LOCAL_CONFIGURATION}")
        })
        .and_then(|asset| {
            let directory = format!("servers/{RELEASE_TAG}");
            install_asset(
                Path::new(&directory),
                &asset,
                |url, path| {
                    zed::set_language_server_installation_status(id, &Status::Downloading);
                    zed::download_file(url, path, DownloadedFileType::Uncompressed)
                },
                zed::make_file_executable,
            )
        });
    let status = match &result {
        Ok(_) => Status::None,
        Err(error) => Status::Failed(error.clone()),
    };
    zed::set_language_server_installation_status(id, &status);
    result
}

fn install_asset(
    directory: &Path,
    asset: &Asset,
    download: impl FnOnce(&str, &str) -> Result<()>,
    make_executable: impl Fn(&str) -> Result<()>,
) -> Result<String> {
    let destination = directory.join(asset.name);
    let command = destination.display().to_string();
    let install = || -> Result<()> {
        if valid_binary(&destination, asset.sha256)? {
            if asset.needs_executable_permission {
                make_executable(&command)?;
            }
            return Ok(());
        }

        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let partial = directory.join(format!("{}.partial", asset.name));
        remove_if_present(&partial)?;
        let partial_path = partial.display().to_string();
        let url = format!("{RELEASES}/{RELEASE_TAG}/{}", asset.name);
        let result = (|| {
            download(&url, &partial_path)?;
            if !valid_binary(&partial, asset.sha256)? {
                return Err("Downloaded file failed SHA-256 verification".into());
            }
            if asset.needs_executable_permission {
                make_executable(&partial_path)?;
            }
            remove_if_present(&destination)?;
            fs::rename(&partial, &destination).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(partial);
        }
        result
    };
    install().map_err(|e| {
        format!("Unable to install Stack language server {RELEASE_TAG}: {e}. {LOCAL_CONFIGURATION}")
    })?;
    Ok(command)
}

fn valid_binary(path: &Path, digest: &str) -> Result<bool> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hasher.finalize()) == digest)
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_verified_binary_and_reuses_it_offline() {
        let directory =
            std::env::temp_dir().join(format!("stack-lsp-cache-{}", std::process::id()));
        let asset = Asset {
            name: "test-server.exe",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            needs_executable_permission: false,
        };
        let destination = directory.join(asset.name);
        let partial = directory.join("test-server.exe.partial");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&destination, b"damaged cache").unwrap();
        fs::write(&partial, b"interrupted download").unwrap();
        let result = install_asset(
            &directory,
            &asset,
            |_, path| {
                assert!(!Path::new(path).exists());
                fs::write(path, b"incorrect download").unwrap();
                Ok(())
            },
            |_| unreachable!("Windows asset does not need chmod"),
        );
        assert!(result.unwrap_err().contains("SHA-256 verification"));
        assert!(!partial.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"damaged cache");

        let installed = install_asset(
            &directory,
            &asset,
            |_, path| {
                fs::write(path, b"abc").unwrap();
                Ok(())
            },
            |_| unreachable!("Windows asset does not need chmod"),
        )
        .unwrap();
        assert_eq!(Path::new(&installed), destination);
        assert_eq!(fs::read(&destination).unwrap(), b"abc");
        assert_eq!(
            install_asset(
                &directory,
                &asset,
                |_, _| panic!("A valid cache must not need a network download"),
                |_| unreachable!("Windows asset does not need chmod"),
            )
            .unwrap(),
            installed
        );
        fs::remove_file(destination).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
