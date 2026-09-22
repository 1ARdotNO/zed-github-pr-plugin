use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result};

struct GithubPrExtension;

impl zed::Extension for GithubPrExtension {
    fn new() -> Self {
        GithubPrExtension
    }

    fn context_server_command(
        &mut self,
        _id: &ContextServerId,
        _project: &Project,
    ) -> Result<Command> {
        // MVP: expect `ghpr-mcp` on PATH. A later revision downloads the
        // per-platform binary from GitHub releases (see TODO Phase 0).
        Ok(Command {
            command: "ghpr-mcp".to_string(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(GithubPrExtension);
