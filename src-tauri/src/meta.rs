//! Serde models for Mojang's version JSON and asset index.
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileRef {
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<FileRef>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, FileRef>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// Maven repository base for libraries that have no `downloads` block.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    pub client: Option<FileRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: FileRef,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logging {
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMeta {
    pub id: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    pub main_class: String,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub asset_index: FileRef,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub logging: Option<Logging>,
    #[serde(default)]
    pub inherits_from: Option<String>,
}

impl VersionMeta {
    /// Java major version required by the game; pre-1.17 versions omit it and need Java 8.
    pub fn required_java_major(&self) -> u32 {
        self.java_version.as_ref().map_or(8, |j| j.major_version)
    }
    pub fn java_component(&self) -> &str {
        self.java_version.as_ref().map_or("jre-legacy", |j| j.component.as_str())
    }
    pub fn asset_index_id(&self) -> &str {
        self.asset_index.id.as_deref().or(self.assets.as_deref()).unwrap_or("legacy")
    }
}

fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    format!("{}:{}:{}", parts.first().unwrap_or(&""), parts.get(1).unwrap_or(&""), parts.get(3).unwrap_or(&""))
}

/// Applies a child profile (e.g. Fabric) on top of its parent. The result keeps the parent's id
/// so the parent's jar, natives and assets are used; libraries of the child win on conflicts.
pub fn overlay(child: VersionMeta, parent: VersionMeta) -> VersionMeta {
    let child_keys: std::collections::HashSet<String> = child.libraries.iter().map(|l| library_key(&l.name)).collect();
    let mut libraries = child.libraries;
    libraries.extend(parent.libraries.into_iter().filter(|l| !child_keys.contains(&library_key(&l.name))));
    let arguments = match (parent.arguments, child.arguments) {
        (Some(mut p), Some(c)) => {
            p.jvm.extend(c.jvm);
            p.game.extend(c.game);
            Some(p)
        }
        (p, c) => p.or(c),
    };
    VersionMeta {
        id: parent.id,
        kind: parent.kind,
        main_class: child.main_class,
        minecraft_arguments: parent.minecraft_arguments,
        arguments,
        asset_index: parent.asset_index,
        assets: parent.assets,
        downloads: parent.downloads,
        libraries,
        java_version: parent.java_version,
        logging: parent.logging,
        inherits_from: None,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    #[serde(default)]
    pub map_to_resources: bool,
}
