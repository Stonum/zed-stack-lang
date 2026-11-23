use std::{fs, io, path::Path};

use sha2::{Digest, Sha256};
use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, LanguageServerId,
    LanguageServerInstallationStatus as Status, Os, Result,
};

const RELEASE_MANIFEST: &str = include_str!("../server-release.json");
const LOCAL_CONFIGURATION: &str =
    "Set lsp.stack-lang-server.binary.path or add stack-lang-server to PATH.";

#[derive(serde::Deserialize)]
struct Release<'a> {
    download_base: &'a str,
    tag: &'a str,
    #[serde(borrow)]
    assets: Assets<'a>,
}

#[derive(serde::Deserialize)]
struct Assets<'a> {
    #[serde(borrow, rename = "windows-x64")]
    windows_x64: Asset<'a>,
    #[serde(borrow, rename = "linux-x64")]
    linux_x64: Asset<'a>,
}

#[derive(serde::Deserialize)]
struct Asset<'a> {
    name: &'a str,
    sha256: &'a str,
}

pub fn install(id: &LanguageServerId) -> Result<String> {
    let result = install_release(id);
    let status = match &result {
        Ok(_) => Status::None,
        Err(error) => Status::Failed(error.clone()),
    };
    zed::set_language_server_installation_status(id, &status);
    result
}

fn install_release(id: &LanguageServerId) -> Result<String> {
    let release: Release<'_> = zed::serde_json::from_str(RELEASE_MANIFEST)
        .map_err(|e| format!("Invalid Stack language server release manifest: {e}"))?;
    let (os, arch) = zed::current_platform();
    let asset = match (os, arch) {
        (Os::Windows, Architecture::X8664) => &release.assets.windows_x64,
        (Os::Linux, Architecture::X8664) => &release.assets.linux_x64,
        _ => {
            return Err(format!(
                "No Stack language server download for this platform. {LOCAL_CONFIGURATION}"
            ));
        }
    };
    let directory = format!("servers/{}", release.tag);
    let url = format!("{}/{}/{}", release.download_base, release.tag, asset.name);
    install_asset(
        Path::new(&directory),
        asset,
        |path| {
            zed::set_language_server_installation_status(id, &Status::Downloading);
            zed::download_file(&url, path, DownloadedFileType::Uncompressed)
        },
        |path| match os {
            Os::Linux => zed::make_file_executable(path),
            _ => Ok(()),
        },
    )
    .map_err(|e| {
        format!(
            "Unable to install Stack language server {}: {e}. {LOCAL_CONFIGURATION}",
            release.tag
        )
    })
}

fn install_asset(
    directory: &Path,
    asset: &Asset<'_>,
    download: impl FnOnce(&str) -> Result<()>,
    make_executable: impl Fn(&str) -> Result<()>,
) -> Result<String> {
    let destination = directory.join(asset.name);
    let command = destination.display().to_string();
    let install = || -> Result<()> {
        if valid_binary(&destination, asset.sha256)? {
            return make_executable(&command);
        }

        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let partial = directory.join(format!("{}.partial", asset.name));
        remove_if_present(&partial)?;
        let partial_path = partial.display().to_string();
        let result = (|| {
            download(&partial_path)?;
            if !valid_binary(&partial, asset.sha256)? {
                return Err("Downloaded file failed SHA-256 verification".into());
            }
            make_executable(&partial_path)?;
            remove_if_present(&destination)?;
            fs::rename(&partial, &destination).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(partial);
        }
        result
    };
    install()?;
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
    Ok(format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(digest))
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
    fn embedded_release_has_digests_for_both_assets() {
        let release: Release<'_> = zed::serde_json::from_str(RELEASE_MANIFEST).unwrap();
        for asset in [release.assets.windows_x64, release.assets.linux_x64] {
            assert_eq!(asset.sha256.len(), 64);
            assert!(asset.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn installs_verified_binary_and_reuses_it_offline() {
        let directory =
            std::env::temp_dir().join(format!("stack-lsp-cache-{}", std::process::id()));
        let asset = Asset {
            name: "test-server.exe",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        };
        let destination = directory.join(asset.name);
        let partial = directory.join("test-server.exe.partial");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&destination, b"damaged cache").unwrap();
        fs::write(&partial, b"interrupted download").unwrap();
        let result = install_asset(
            &directory,
            &asset,
            |path| {
                assert!(!Path::new(path).exists());
                fs::write(path, b"incorrect download").unwrap();
                Ok(())
            },
            |_| Ok(()),
        );
        assert!(result.unwrap_err().contains("SHA-256 verification"));
        assert!(!partial.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"damaged cache");

        let installed = install_asset(
            &directory,
            &asset,
            |path| {
                fs::write(path, b"abc").unwrap();
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(Path::new(&installed), destination);
        assert_eq!(fs::read(&destination).unwrap(), b"abc");
        assert_eq!(
            install_asset(
                &directory,
                &asset,
                |_| panic!("A valid cache must not need a network download"),
                |_| Ok(()),
            )
            .unwrap(),
            installed
        );
        fs::remove_file(destination).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
