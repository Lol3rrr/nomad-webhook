use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GithubPackagePayload {
    pub action: String,
    pub package: GithubPackage,
    pub repository: serde_json::Value,
    pub sender: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct GithubPackage {
    pub id: usize,
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub package_type: String,
    pub package_version: GithubPackageVersion,
    pub registry: GithubPacakgeRegistry,
}

#[derive(Debug, Deserialize)]
pub struct GithubPackageVersion {
    pub name: String,
    pub version: String,
    pub container_metadata: GithubContainerMetadata,
    pub package_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubContainerMetadata {
    pub tag: GithubContainerTag,
}

#[derive(Debug, Deserialize)]
pub struct GithubContainerTag {
    pub name: String,
    pub digest: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubPacakgeRegistry {
    pub name: String,
    pub url: String,
    pub vendor: String,
}

impl GithubPackagePayload {
    pub fn is_package(raw: &serde_json::Value) -> bool {
        let obj = match raw.as_object() {
            Some(o) => o,
            None => return false,
        };

        let raw_action = match obj.get("action") {
            Some(ra) => ra,
            None => return false,
        };

        let action = match raw_action.as_str() {
            Some(a) => a,
            None => return false,
        };

        action.eq_ignore_ascii_case("published") || action.eq_ignore_ascii_case("updated")
    }

    pub fn into_package(raw: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(raw)
    }
}
