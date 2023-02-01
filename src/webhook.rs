use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GithubPackagePayload {
    action: String,
    package: GithubPackage,
    repository: serde_json::Value,
    sender: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct GithubPackage {
    name: String,
    namespace: String,
    package_type: String,
    package_version: GithubPackageVersion,
    registry: GithubPacakgeRegistry,
}

#[derive(Debug, Deserialize)]
pub struct GithubPackageVersion {
    name: String,
    tag_name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubPacakgeRegistry {
    name: String,
    url: String,
    vendor: String,
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

    pub fn into_package(raw: serde_json::Value) -> Result<Self, ()> {
        serde_json::from_value(raw).map_err(|_| ())
    }
}
