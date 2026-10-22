use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

pub const PRODUCTS: [&str; 6] = [
    "Code",
    "Code - Insiders",
    "VSCodium",
    "Cursor",
    "Windsurf",
    "Trae",
];

pub struct Environment {
    pub home: PathBuf,
    pub app_roots: Vec<PathBuf>,
    pub isolated: bool,
}

impl Environment {
    pub fn load() -> Result<Self> {
        let isolated = std::env::var_os("SESSION_ARCHAEOLOGY_HOME");
        let home = isolated
            .clone()
            .or_else(|| std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }))
            .context("home directory unavailable; set SESSION_ARCHAEOLOGY_HOME")?;
        let home = crate::files::absolute(&PathBuf::from(home))?;
        let mut app_roots = vec![
            home.join("Library/Application Support"),
            home.join(".config"),
        ];
        if isolated.is_none() {
            for name in ["APPDATA", "XDG_CONFIG_HOME"] {
                if let Some(root) = std::env::var_os(name) {
                    app_roots.push(PathBuf::from(root));
                }
            }
        } else {
            app_roots.push(home.join("AppData/Roaming"));
        }
        app_roots.sort();
        app_roots.dedup();
        Ok(Self {
            home,
            app_roots,
            isolated: isolated.is_some(),
        })
    }
    pub fn vscode_stores(&self) -> Vec<PathBuf> {
        self.app_roots
            .iter()
            .flat_map(|root| {
                PRODUCTS
                    .iter()
                    .map(move |product| root.join(product).join("User/workspaceStorage"))
            })
            .collect()
    }
    pub fn doctor(&self) -> Value {
        let supported = vec![
            (
                "claude-code",
                vec![
                    self.home.join(".claude/projects"),
                    self.home.join(".claude/history.jsonl"),
                ],
            ),
            (
                "dsh",
                vec![
                    self.home.join(".dsh/sessions"),
                    self.home.join(".dsh/storages/workspace.json"),
                ],
            ),
            ("codex", vec![self.home.join(".codex/sessions")]),
            ("copilot", self.vscode_stores()),
        ];
        let sources:Vec<_>=supported.into_iter().map(|(name,paths)|json!({"source":name,"available":paths.iter().any(|p|p.exists()),"paths":paths.iter().map(|p|json!({"path":p,"exists":p.exists()})).collect::<Vec<_>>()})).collect();
        let unsupported:Vec<_>=[("MAgent",".magent"),("CatPaw",".catpaw"),("CatDesk",".catdesk"),("Gemini CLI",".gemini"),("Continue",".continue"),("OpenCode",".local/share/opencode"),("Zed",".local/share/zed")].iter().map(|(name,path)|json!({"source":name,"supported":false,"probe":self.home.join(path),"present":self.home.join(path).exists()})).collect();
        json!({"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"arch":std::env::consts::ARCH,"mode":"offline","auth":{"required":false,"source":"not_required"},"network_required":false,"home":self.home,"isolated_stores":self.isolated,"data_found":sources.iter().any(|s|s["available"]==true),"sources":sources,"unsupported":unsupported,"zstd":"built_in_multi_frame","note":"Presence is a probe, not proof of parse coverage. Unsupported probes are not exhaustive."})
    }
}
