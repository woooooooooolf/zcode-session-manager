use std::process::Command;

fn main() {
    generate_third_party();
    tauri_build::build();
}

/// Generate a (name, version, license) table of the workspace's direct
/// dependencies at build time by asking cargo for workspace metadata.
/// Falls back to an empty table (flagged unavailable) if cargo metadata
/// cannot run — the UI shows a note instead of a table then.
fn generate_third_party() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let output = Command::new(&cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output();

    let (table, generated) = match output {
        Ok(o) if o.status.success() => match parse_metadata(&o.stdout) {
            Some(t) => (t, true),
            None => (Vec::new(), false),
        },
        _ => (Vec::new(), false),
    };

    let mut src = String::new();
    src.push_str(&format!("pub const THIRD_PARTY_GENERATED: bool = {generated};\n"));
    src.push_str("pub const THIRD_PARTY: &[(&str, &str, &str)] = &[\n");
    for (name, version, license) in &table {
        src.push_str(&format!("    ({name:?}, {version:?}, {license:?}),\n"));
    }
    src.push_str("];\n");

    let dest = std::path::Path::new(&out_dir).join("third_party.rs");
    std::fs::write(&dest, src).expect("write third_party.rs");
}

/// Direct dependencies of every workspace member (normal + build kinds),
/// excluding the members themselves, resolved to (name, version, license).
fn parse_metadata(bytes: &[u8]) -> Option<Vec<(String, String, String)>> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let packages = v.get("packages")?.as_array()?;
    let members: Vec<&str> = v
        .get("workspace_members")?
        .as_array()?
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    let member_names: Vec<&str> = packages
        .iter()
        .filter(|p| {
            p.get("id")
                .and_then(|x| x.as_str())
                .map(|id| members.contains(&id))
                .unwrap_or(false)
        })
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
        .collect();

    let mut direct: Vec<String> = Vec::new();
    for p in packages {
        let id = p.get("id")?.as_str()?;
        if !members.contains(&id) {
            continue;
        }
        if let Some(deps) = p.get("dependencies").and_then(|d| d.as_array()) {
            for d in deps {
                // kind: null = normal, "dev"/"build" otherwise; skip dev
                let kind = d.get("kind").and_then(|k| k.as_str());
                if kind == Some("dev") {
                    continue;
                }
                if let Some(name) = d.get("name").and_then(|n| n.as_str()) {
                    if !member_names.contains(&name) && !direct.iter().any(|x| x == name) {
                        direct.push(name.to_string());
                    }
                }
            }
        }
    }
    direct.sort();

    let mut table = Vec::new();
    for name in direct {
        if let Some(p) = packages
            .iter()
            .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(name.as_str()))
        {
            let version = p.get("version").and_then(|x| x.as_str()).unwrap_or("?").to_string();
            let license = p
                .get("license")
                .and_then(|x| x.as_str())
                .unwrap_or("see crate")
                .to_string();
            table.push((name, version, license));
        }
    }
    Some(table)
}
