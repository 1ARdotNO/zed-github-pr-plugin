use zed_extension_api::{
    self as zed, Architecture, Command, ContextServerId, DownloadedFileType, GithubReleaseOptions,
    Os, Project, Result,
};

const REPO: &str = "1ARdotNO/zed-github-pr-plugin";
const BIN: &str = "ghpr-mcp";

struct GithubPrExtension {
    cached_binary_path: Option<String>,
}

impl GithubPrExtension {
    /// Resolve the `ghpr-mcp` binary, downloading the latest release asset for
    /// this platform on first use and caching it. Errors bubble up so the caller
    /// can fall back to `PATH`.
    fn download_binary(&mut self) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        let target = target_triple()?;
        let release = zed::latest_github_release(
            REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;
        let asset_name = format!("{BIN}-{target}.tar.gz");
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset named {asset_name}"))?;

        // taiki-e's tarball holds the binary at the archive root.
        let version_dir = format!("{BIN}-{}", release.version);
        let binary_path = format!("{version_dir}/{BIN}");
        if !std::fs::metadata(&binary_path).is_ok_and(|m| m.is_file()) {
            zed::download_file(
                &asset.download_url,
                &version_dir,
                DownloadedFileType::GzipTar,
            )?;
            zed::make_file_executable(&binary_path)?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

/// The Rust target triple for the current platform, matching release asset names.
fn target_triple() -> Result<&'static str> {
    let (os, arch) = zed::current_platform();
    Ok(match (os, arch) {
        (Os::Mac, Architecture::Aarch64) => "aarch64-apple-darwin",
        (Os::Mac, Architecture::X8664) => "x86_64-apple-darwin",
        (Os::Linux, Architecture::X8664) => "x86_64-unknown-linux-gnu",
        (Os::Linux, Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
        _ => return Err("no prebuilt ghpr-mcp for this platform".into()),
    })
}

impl zed::Extension for GithubPrExtension {
    fn new() -> Self {
        GithubPrExtension {
            cached_binary_path: None,
        }
    }

    fn context_server_command(
        &mut self,
        _id: &ContextServerId,
        _project: &Project,
    ) -> Result<Command> {
        // Prefer a downloaded release binary; fall back to `ghpr-mcp` on PATH so
        // the extension also works before any release exists (or from source).
        let command = self.download_binary().unwrap_or_else(|_| BIN.to_string());
        Ok(Command {
            command,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(GithubPrExtension);
