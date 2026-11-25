//! Lake's evaluated package layout plus the exact host-observed metadata.
use crate::atlas::{
    lake::{Dependency, Glob, Package},
    Configuration,
};
use crate::workspace::observation::{relative, Entry, Reader};
use anyhow::{Context, Result};
use meden_runtime::process::Control;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
};

pub(super) fn source_entry(entry: &Entry) -> bool {
    entry.kind != "file"
        || Path::new(&entry.name)
            .extension()
            .is_some_and(|ext| ext == "lean")
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub name: String,
    pub file: String,
    pub root: String,
}
impl Module {
    pub fn validate(&self) -> Result<()> {
        let path = format!("{}.lean", self.name.replace('.', "/"));
        crate::atlas::module_path(&path)?;
        anyhow::ensure!(
            relative("", &self.root)? == self.root && relative(&self.root, &path)? == self.file,
            "Module/source-root attribution is inconsistent"
        );
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PackageView {
    pub name: String,
    pub directory: String,
    pub source_roots: Vec<String>,
}
pub(super) struct Layout {
    pub packages: Vec<PackageView>,
    directories: BTreeSet<String>,
    libraries: Vec<crate::atlas::lake::LibrarySource>,
    pub globs: Vec<Glob>,
}
impl Layout {
    pub fn discover(
        reader: &mut Reader<'_>,
        atlas: &Configuration,
        control: &Control,
        selected: &[String],
    ) -> Result<Self> {
        let _timing = meden_runtime::metrics::Span::start("analysis.discover_layout");
        let config = configuration(reader, "", None)?;
        let directory = if config.is_some() {
            String::new()
        } else {
            let mut candidate = selected.first().and_then(|p| Path::new(p).parent());
            let mut found = None;
            while let Some(path) = candidate {
                let directory = path.to_str().context("Lake directory is not UTF-8")?;
                if configuration(reader, directory, None)?.is_some() {
                    found = Some(directory.to_owned());
                    break;
                }
                candidate = path.parent();
            }
            let Some(found) = found else {
                return Ok(Self {
                    packages: Vec::new(),
                    directories: BTreeSet::from([String::new()]),
                    libraries: Vec::new(),
                    globs: Vec::new(),
                });
            };
            found
        };
        let config =
            configuration(reader, &directory, None)?.context("Lake configuration disappeared")?;
        let root = atlas.lake_package(
            &reader.metadata,
            &directory,
            &config,
            &BTreeMap::new(),
            control,
        )?;
        let manifest_path = relative(&directory, &root.manifest)?;
        let manifest = if reader.read(&manifest_path, true)?.is_some() {
            Some(atlas.lake_manifest(&reader.metadata, &manifest_path, control)?)
        } else {
            None
        };
        let entries = manifest_entries(manifest.as_ref(), &root.name)?;
        let packages_directory = relative(
            &directory,
            manifest
                .as_ref()
                .and_then(|m| m.get("packagesDir"))
                .and_then(Value::as_str)
                .unwrap_or(&root.packages_dir),
        )?;
        let mut pending = VecDeque::from([(directory.clone(), root, BTreeMap::new())]);
        let mut seen = BTreeMap::new();
        let mut layout = Self {
            packages: Vec::new(),
            directories: BTreeSet::new(),
            libraries: Vec::new(),
            globs: Vec::new(),
        };
        while let Some((dir, pkg, options)) = pending.pop_front() {
            anyhow::ensure!(
                layout.packages.len() < 64,
                "Lake dependency graph exceeds 64 packages"
            );
            if let Some(previous) = seen.insert(pkg.name.clone(), (dir.clone(), options.clone())) {
                anyhow::ensure!(
                    previous == (dir, options),
                    "Conflicting Lake package identity/options"
                );
                continue;
            }
            let children = pkg
                .dependencies
                .iter()
                .map(|dep| {
                    dependency_location(&dir, &directory, &packages_directory, dep, &entries)
                })
                .collect::<Result<Vec<_>>>()?;
            prefetch_dependencies(reader, &children)?;
            for (dep, (child, config)) in pkg.dependencies.iter().zip(children) {
                let config = configuration(reader, &child, config.as_deref())?.with_context(|| {
                    let preparation = if dep.name == "mathlib" {
                        "For a Mathlib project, run `lake exe cache get` in the project directory before retrying."
                    } else {
                        "Prepare this dependency using the project's README before retrying."
                    };
                    format!("Lake dependency {} is not available at {child}.\n{preparation}\nDependency preparation is separate from offline analysis; no fetching or partial graph was performed. To inspect files now, use the source-only graph.", dep.name)
                })?;
                let child_pkg =
                    atlas.lake_package(&reader.metadata, &child, &config, &dep.options, control)?;
                anyhow::ensure!(
                    child_pkg.name == dep.name,
                    "Lake dependency name does not match its package configuration"
                );
                pending.push_back((child, child_pkg, dep.options.clone()));
            }
            layout.add_package(&dir, &pkg, dir == directory)?;
        }
        Ok(layout)
    }
    fn add_package(&mut self, directory: &str, pkg: &Package, automatic: bool) -> Result<()> {
        let roots = package_roots(directory, pkg)?;
        if automatic {
            for glob in &pkg.globs {
                let mut glob = glob.clone();
                glob.directory = relative(&relative(directory, &pkg.src_dir)?, &glob.directory)?;
                self.globs.push(glob);
            }
        }
        for library in &pkg.library_sources {
            let mut library = library.clone();
            library.directory = relative(&relative(directory, &pkg.src_dir)?, &library.directory)?;
            self.libraries.push(library);
        }
        self.directories.extend(roots.iter().cloned());
        self.packages.push(PackageView {
            name: pkg.name.clone(),
            directory: directory.into(),
            source_roots: roots,
        });
        Ok(())
    }
    pub fn automatic_sources(&self, reader: &mut Reader<'_>) -> Result<BTreeMap<String, String>> {
        let mut files = BTreeMap::new();
        for glob in &self.globs {
            anyhow::ensure!(
                matches!(glob.kind.as_str(), "one" | "submodules" | "andSubmodules"),
                "Unknown Lake module glob"
            );
            let path = relative(&glob.directory, &glob.name.replace('.', "/"))?;
            if glob.kind != "submodules" && !glob.name.is_empty() {
                let file = format!("{path}.lean");
                if let Some(source) = reader.read(&file, false)? {
                    files.insert(file, source);
                }
            }
            if glob.kind != "one" {
                walk(reader, &path, &mut files)?;
            }
        }
        anyhow::ensure!(
            files.len() <= 32,
            "Lake project exceeds 32 selected source modules; select an explicit root file"
        );
        Ok(files)
    }
    pub fn bind_selected(&self, file: &str) -> Result<Module> {
        let mut roots = self
            .directories
            .iter()
            .filter(|root| root.is_empty() || file.starts_with(&format!("{root}/")))
            .collect::<Vec<_>>();
        let mut owned = BTreeMap::new();
        for library in &self.libraries {
            let root = library.directory.as_str();
            let path = if root.is_empty() {
                Some(file)
            } else {
                file.strip_prefix(&format!("{root}/"))
            };
            if let Some(path) = path.and_then(|path| path.strip_suffix(".lean")) {
                let name = path.replace('/', ".");
                if library
                    .selectors
                    .iter()
                    .any(|selector| matches(selector, &name))
                {
                    owned.insert(
                        name.clone(),
                        Module {
                            name,
                            file: file.into(),
                            root: root.into(),
                        },
                    );
                }
            }
        }
        if !owned.is_empty() {
            anyhow::ensure!(
                owned.len() == 1,
                "Selected source has ambiguous Lake library module identities"
            );
            let module = owned
                .into_values()
                .next()
                .context("Missing owned Lake module")?;
            module.validate()?;
            return Ok(module);
        }
        roots.sort_by_key(|root| std::cmp::Reverse(root.len()));
        let root = roots
            .first()
            .context("Selected source is outside Lake's configured source directories")?;
        let path = if root.is_empty() {
            file
        } else {
            file.strip_prefix(&format!("{root}/"))
                .context("Source root mismatch")?
        };
        let name = path
            .strip_suffix(".lean")
            .context("Lake analysis requires a Lean source module")?
            .replace('/', ".");
        let module = Module {
            name,
            file: file.into(),
            root: (*root).clone(),
        };
        module.validate()?;
        Ok(module)
    }
    pub fn resolve(&self, name: &str, reader: &mut Reader<'_>) -> Result<(Module, String)> {
        let path = format!("{}.lean", name.replace('.', "/"));
        crate::atlas::module_path(&path)?;
        let mut candidates = BTreeMap::new();
        let owned = self
            .libraries
            .iter()
            .filter(|lib| lib.selectors.iter().any(|selector| matches(selector, name)))
            .map(|lib| lib.directory.clone())
            .collect::<BTreeSet<_>>();
        let roots = if owned.is_empty() {
            &self.directories
        } else {
            &owned
        };
        for root in roots {
            let file = relative(root, &path)?;
            if let Some(source) = reader.read(&file, false)? {
                candidates.insert(
                    file.clone(),
                    (
                        Module {
                            name: name.into(),
                            file,
                            root: root.clone(),
                        },
                        source,
                    ),
                );
            }
        }
        anyhow::ensure!(candidates.len() == 1, "Project-local import collection failed: module {name} has {} source candidates (missing or ambiguous); no partial graph", candidates.len());
        candidates
            .into_values()
            .next()
            .context("Missing module source")
    }
}
fn prefetch_dependencies(
    reader: &mut Reader<'_>,
    children: &[(String, Option<String>)],
) -> Result<()> {
    let upcoming = children
        .iter()
        .map(|(child, config)| configuration_paths(child, config.as_deref()))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map(|(_, path)| path)
        .collect::<Vec<_>>();
    reader.prefetch_files(&upcoming)
}
fn configuration(
    reader: &mut Reader<'_>,
    directory: &str,
    explicit: Option<&str>,
) -> Result<Option<String>> {
    let mut chosen = None;
    for (name, path) in configuration_paths(directory, explicit)? {
        if reader.read(&path, true)?.is_some() && chosen.is_none() {
            chosen = Some(name);
        }
    }
    Ok(chosen)
}
fn configuration_paths(directory: &str, explicit: Option<&str>) -> Result<Vec<(String, String)>> {
    let names = if let Some(explicit) = explicit {
        if Path::new(explicit).extension().is_some() {
            vec![explicit.to_owned()]
        } else {
            vec![format!("{explicit}.lean"), format!("{explicit}.toml")]
        }
    } else {
        vec!["lakefile.lean".into(), "lakefile.toml".into()]
    };
    names
        .into_iter()
        .map(|name| {
            let path = relative(directory, &name)?;
            Ok((name, path))
        })
        .collect()
}
fn package_roots(directory: &str, package: &Package) -> Result<Vec<String>> {
    let src = relative(directory, &package.src_dir)?;
    let mut roots = BTreeSet::from([src.clone()]);
    for child in package
        .library_sources
        .iter()
        .map(|lib| &lib.directory)
        .chain(&package.executable_dirs)
    {
        roots.insert(relative(&src, child)?);
    }
    Ok(roots.into_iter().collect())
}
fn manifest_entries(manifest: Option<&Value>, name: &str) -> Result<BTreeMap<String, Value>> {
    let Some(manifest) = manifest else {
        return Ok(BTreeMap::new());
    };
    anyhow::ensure!(
        manifest.get("name").and_then(Value::as_str) == Some(name),
        "Lake manifest belongs to another package"
    );
    let rows = manifest
        .get("packages")
        .and_then(Value::as_array)
        .context("Lake manifest has no packages")?;
    anyhow::ensure!(rows.len() <= 256, "Lake manifest exceeds 256 packages");
    let mut entries = BTreeMap::new();
    for row in rows {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .context("Missing manifest package name")?;
        anyhow::ensure!(
            entries.insert(name.into(), row.clone()).is_none(),
            "Duplicate Lake manifest package name"
        );
    }
    Ok(entries)
}
fn dependency_location(
    parent: &str,
    workspace: &str,
    packages: &str,
    dep: &Dependency,
    entries: &BTreeMap<String, Value>,
) -> Result<(String, Option<String>)> {
    anyhow::ensure!(
        !dep.name.is_empty() && dep.name != "." && dep.name != ".." && !dep.name.contains('/'),
        "Invalid Lake package name"
    );
    let entry = entries.get(&dep.name);
    let config = entry
        .and_then(|e| e.get("configFile"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let directory = if dep.kind == "path" {
        if let Some(entry) = entry {
            anyhow::ensure!(
                entry.get("type").and_then(Value::as_str) == Some("path"),
                "Manifest/source kind conflict"
            );
            let declared = relative(parent, &dep.path)?;
            anyhow::ensure!(
                relative(
                    workspace,
                    entry
                        .get("dir")
                        .and_then(Value::as_str)
                        .context("Missing manifest dependency path")?
                )? == declared,
                "Manifest path differs from declared dependency"
            );
        }
        relative(parent, &dep.path)?
    } else {
        let entry = entry.context("Lake dependency needs its installed package and root manifest; automatic fetching is disabled")?;
        anyhow::ensure!(
            matches!(dep.kind.as_str(), "git" | "copy" | "registry"),
            "Unknown Lake dependency source"
        );
        if dep.kind == "registry"
            && entry.get("type").and_then(Value::as_str) == Some("path")
            && entry.get("copy").and_then(Value::as_bool) != Some(true)
        {
            let path = entry
                .get("dir")
                .and_then(Value::as_str)
                .context("Missing manifest path override")?;
            return Ok((relative(workspace, path)?, config));
        }
        if dep.kind == "git" {
            anyhow::ensure!(
                entry.get("type").and_then(Value::as_str) == Some("git")
                    && entry.get("url").and_then(Value::as_str) == Some(&dep.url),
                "Manifest Git source differs from the package declaration"
            );
            anyhow::ensure!(
                entry.get("subDir").and_then(Value::as_str).unwrap_or("") == dep.subdir,
                "Manifest Git subdirectory differs from the package declaration"
            );
            if let Some(revision) = &dep.revision {
                anyhow::ensure!(
                    entry.get("inputRev").and_then(Value::as_str) == Some(revision)
                        || entry.get("rev").and_then(Value::as_str) == Some(revision),
                    "Manifest Git revision differs from the package declaration"
                );
            }
        }
        if dep.kind == "copy" {
            anyhow::ensure!(
                entry.get("type").and_then(Value::as_str) == Some("path")
                    && entry.get("copy").and_then(Value::as_bool) == Some(true),
                "Manifest copy source differs from the package declaration"
            );
        }
        let base = relative(packages, &dep.name)?;
        let subdir = entry.get("subDir").and_then(Value::as_str).unwrap_or("");
        relative(&base, subdir)?
    };
    Ok((directory, config))
}

fn walk(
    reader: &mut Reader<'_>,
    directory: &str,
    files: &mut BTreeMap<String, String>,
) -> Result<()> {
    for entry in reader.observe_directory(directory)? {
        let path = relative(directory, &entry.name)?;
        match entry.kind.as_str() {
            "directory" => walk(reader, &path, files)?,
            "file" | "symlink" if Path::new(&entry.name).extension().is_some_and(|ext|ext == "lean") => {
                if let Some(source) = reader.read(&path, false)? { files.insert(path, source); }
            },
            "symlink" => anyhow::bail!("Lake automatic module discovery cannot attribute directory symlink {path}; select an explicit source root"),
            "other" => anyhow::bail!("Lake module discovery encountered a special filesystem entry at {path}"),
            _ => {},
        }
        anyhow::ensure!(
            files.len() <= 32,
            "Lake project exceeds 32 selected source modules; select an explicit root file"
        );
    }
    Ok(())
}

fn matches(selector: &crate::atlas::lake::Glob, name: &str) -> bool {
    let child = selector.name.is_empty() || name.starts_with(&format!("{}.", selector.name));
    match selector.kind.as_str() {
        "one" => name == selector.name,
        "submodules" => child,
        "andSubmodules" => name == selector.name || child,
        _ => false,
    }
}
