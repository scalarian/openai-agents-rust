use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn parse_family_rows(source: &str) -> BTreeMap<String, (String, Vec<String>)> {
    source
        .lines()
        .filter(|line| line.trim_start().starts_with("| `"))
        .filter_map(|line| {
            let columns = line
                .split('|')
                .map(str::trim)
                .filter(|column| !column.is_empty())
                .collect::<Vec<_>>();
            if columns.len() < 4 {
                return None;
            }

            let family = columns[0].trim_matches('`').to_owned();
            let status = columns[1].trim_matches('`').to_owned();
            let coverage_paths = columns[2]
                .split(',')
                .map(str::trim)
                .map(|path| path.trim_matches('`'))
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            Some((family, (status, coverage_paths)))
        })
        .collect()
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn upstream_python_families(root: &Path) -> BTreeSet<String> {
    let tests_root = root.join("reference/openai-agents-python/tests");
    let mut files = Vec::new();
    collect_files(&tests_root, &mut files);
    files
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("py"))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("test_"))
        })
        .map(|path| {
            path.strip_prefix(&tests_root)
                .expect("python test relative path")
                .with_extension("")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn upstream_js_families(root: &Path) -> BTreeSet<String> {
    let packages_root = root.join("reference/openai-agents-js/packages");
    let mut files = Vec::new();
    collect_files(&packages_root, &mut files);
    files
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("ts"))
        .filter_map(|path| {
            let relative = path.strip_prefix(&packages_root).ok()?;
            let relative_text = relative.to_string_lossy().replace('\\', "/");
            let (package, rest) = relative_text.split_once('/')?;
            let test_prefix = "test/";
            let rest = rest.strip_prefix(test_prefix)?;
            let suffix = ".test.ts";
            let family = rest.strip_suffix(suffix)?;
            Some(format!("js/{package}/{family}"))
        })
        .collect()
}

#[test]
fn behavior_parity_doc_covers_upstream_family_inventory() {
    let root = workspace_root();
    let parity_doc =
        fs::read_to_string(root.join("docs/BEHAVIOR_PARITY.md")).expect("behavior parity doc");
    let families = parse_family_rows(&parity_doc);
    let documented = families.keys().cloned().collect::<BTreeSet<_>>();
    let expected = upstream_python_families(&root)
        .into_iter()
        .chain(upstream_js_families(&root))
        .collect::<BTreeSet<_>>();

    let missing = expected
        .difference(&documented)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = documented
        .difference(&expected)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "Missing behavior parity families: {}",
        missing.join(", ")
    );
    assert!(
        unexpected.is_empty(),
        "Behavior parity doc contains unexpected families: {}",
        unexpected.join(", ")
    );
}

#[test]
fn behavior_parity_doc_uses_allowed_statuses_and_existing_paths() {
    let root = workspace_root();
    let parity_doc =
        fs::read_to_string(root.join("docs/BEHAVIOR_PARITY.md")).expect("behavior parity doc");
    let families = parse_family_rows(&parity_doc);
    let allowed_statuses = ["covered", "omitted-with-rationale"];

    let mut invalid_statuses = Vec::new();
    let mut missing_paths = Vec::new();
    let mut partial_families = Vec::new();

    for (family, (status, paths)) in families {
        if status == "partial" {
            partial_families.push(family.clone());
        }
        if !allowed_statuses.contains(&status.as_str()) {
            invalid_statuses.push(format!("{family} -> {status}"));
        }
        if status != "omitted-with-rationale" {
            for path in paths {
                if !root.join(&path).exists() {
                    missing_paths.push(format!("{family} -> {path}"));
                }
            }
        }
    }

    assert!(
        invalid_statuses.is_empty(),
        "Behavior parity doc contains invalid statuses: {}",
        invalid_statuses.join(", ")
    );
    assert!(
        partial_families.is_empty(),
        "Behavior parity doc still contains partial families: {}",
        partial_families.join(", ")
    );
    assert!(
        missing_paths.is_empty(),
        "Behavior parity doc references missing coverage paths: {}",
        missing_paths.join(", ")
    );
}
