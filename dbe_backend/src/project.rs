use crate::etype::EDataType;
use crate::graph::execution::GraphExecutionContext;
use crate::json_utils::formatter::DBEJsonFormatter;
use crate::json_utils::{json_kind, JsonValue};
use crate::m_try;
use crate::project::docs::{Docs, DocsFile};
use crate::project::io::{FilesystemIO, ProjectIO};
use crate::project::project_graph::{ProjectGraph, ProjectGraphs};
use crate::project::side_effects::SideEffectsContext;
use crate::project::undo::{UndoHistory, UndoSettings};
use crate::registry::ETypesRegistry;
use crate::validation::{clear_validation_cache, validate};
use crate::value::id::ETypeId;
use crate::value::EValue;
use camino::{Utf8Path, Utf8PathBuf};
use diagnostic::context::DiagnosticContext;
use diagnostic::diagnostic::DiagnosticLevel;
use miette::{bail, miette, Context, IntoDiagnostic, Report};
use rayon::iter::ParallelDrainFull;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;
use utils::map::HashSet;
use uuid::Uuid;
use walkdir::WalkDir;

pub mod docs;
pub mod io;
pub mod project_graph;
pub mod side_effects;
pub mod undo;

pub const EXTENSION_GRAPH: &str = "dbegraph";
pub const EXTENSION_VALUE: &str = "dbevalue";
pub const EXTENSION_ITEM: &str = "json";
pub const EXTENSION_DOCS: &str = "docs.toml";

#[derive(Debug)]
pub struct Project<IO> {
    /// Types registry
    pub registry: ETypesRegistry,
    pub docs: Docs,
    /// Diagnostic context
    pub diagnostics: DiagnosticContext,
    /// Files present in the project
    pub files: BTreeMap<Utf8PathBuf, ProjectFile>,
    pub graphs: ProjectGraphs,
    /// Files that should be deleted on save
    pub to_delete: HashSet<Utf8PathBuf>,
    pub history: UndoHistory,
    /// Root folder of the project
    pub root: Utf8PathBuf,
    io: IO,
}

#[derive(Debug)]
pub enum ProjectFile {
    /// Valid plain JSON value
    Value(EValue),
    /// Valid plain JSON value that was automatically generated
    GeneratedValue(EValue),
    /// Snarl graph
    Graph(Uuid),
    /// Plain JSON value that had issues during parsing or loading
    BadValue(Report),
}

#[derive(Debug, Serialize, Deserialize)]
struct MiscJson {
    ty: EDataType,
    value: JsonValue,
}

impl ProjectFile {
    pub fn is_value(&self) -> bool {
        matches!(self, ProjectFile::Value(..))
    }

    pub fn is_generated(&self) -> bool {
        matches!(self, ProjectFile::GeneratedValue(..))
    }

    pub fn is_graph(&self) -> bool {
        matches!(self, ProjectFile::Graph(_))
    }

    pub fn is_bad(&self) -> bool {
        matches!(self, ProjectFile::BadValue(_))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(rename = "types")]
    pub types_config: TypesConfig,
    #[serde(default = "default_emitted_dir")]
    pub emitted_dir: Utf8PathBuf,
}

fn default_emitted_dir() -> Utf8PathBuf {
    Utf8PathBuf::from("emitted")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypesConfig {
    root: String,
    pub import: ETypeId,
}

impl Project<FilesystemIO> {
    pub fn from_path(root: impl AsRef<Path>) -> miette::Result<Self> {
        let root = root.as_ref();

        let mut paths = BTreeSet::new();
        let wd = WalkDir::new(root);
        for entry in wd {
            let entry = entry.into_diagnostic()?;
            if entry.path().is_dir() {
                continue;
            }

            paths.insert(entry.path().to_path_buf());
        }

        let config = fs_err::read_to_string(root.join("project.toml"))
            .into_diagnostic()
            .context("failed to read project configuration")?;

        let config = toml::de::from_str(&config)
            .into_diagnostic()
            .context("Failed to parse project configuration")?;

        // let items = paths
        //     .into_par_iter()
        //     .map(|path| {
        //         let data = fs_err::read(&path).into_diagnostic()?;
        //         miette::Result::<(PathBuf, Vec<u8>)>::Ok((path, data))
        //     })
        //     .collect::<Result<Vec<_>, _>>()?;

        Self::from_files(root, config, paths, FilesystemIO::new(root.to_path_buf()))
    }
}

impl<IO: ProjectIO> Project<IO> {
    pub fn from_files(
        root: impl AsRef<Path>,
        config: ProjectConfig,
        files: impl IntoIterator<Item = PathBuf>,
        mut io: IO,
    ) -> miette::Result<Self> {
        let mut registry_items = BTreeMap::new();
        let mut import_jsons = BTreeMap::<Utf8PathBuf, (JsonValue, Option<EDataType>)>::new();
        let mut types_jsons = BTreeMap::<Utf8PathBuf, JsonValue>::new();
        let mut graphs = BTreeMap::<Utf8PathBuf, JsonValue>::new();
        let mut docs = Docs::Docs(Default::default());

        fn utf8str(path: &Utf8Path, data: Vec<u8>) -> miette::Result<String> {
            String::from_utf8(data).into_diagnostic().with_context(|| {
                format!(
                    "failed to parse content of a file `{path}`. Are you sure it's UTF-8 encoded?"
                )
            })
        }

        let root = root.as_ref();
        let root = Utf8PathBuf::from_path_buf(root.to_path_buf())
            .map_err(|_| miette!("Got non-UTF8 path at {}", root.display()))?;

        for path in files {
            let relative = path
                .strip_prefix(&root).map_err(|_| miette!("directory contains file `{}` which is outside of the directory. Are there symlinks?", path.display()))?;

            let path = Utf8Path::from_path(relative)
                .ok_or_else(|| miette!("Got non-UTF8 path at {}", relative.display()))?;
            let lower_path = path.as_str().to_lowercase();

            let Some(ext) = path.extension() else {
                continue;
            };

            m_try(|| {
                match ext {
                    "kdl" => {
                        let id = ETypeId::from_path(path, &config.types_config.root)
                            .context("failed to generate type identifier")?;
                        let value = utf8str(path, io.read_file(path)?)?;
                        registry_items.insert(id, value);
                    }
                    "json5" | "json" => {
                        let data = serde_json5::from_str(&utf8str(path, io.read_file(path)?)?)
                            .into_diagnostic()
                            .context("failed to deserialize JSON")?;
                        if path.starts_with(&config.types_config.root) {
                            types_jsons.insert(path.to_path_buf(), data);
                        } else {
                            import_jsons.insert(path.to_path_buf(), (data, None));
                        }
                    }
                    EXTENSION_VALUE => {
                        let data: MiscJson =
                            serde_json5::from_str(&utf8str(path, io.read_file(path)?)?)
                                .into_diagnostic()
                                .context("failed to deserialize dbefile JSON")?;
                        import_jsons.insert(path.to_path_buf(), (data.value, Some(data.ty)));
                    }
                    EXTENSION_GRAPH => {
                        let data = serde_json5::from_str(&utf8str(path, io.read_file(path)?)?)
                            .into_diagnostic()
                            .context("failed to deserialize graph JSON")?;
                        graphs.insert(path.to_path_buf(), data);
                    }
                    "toml" if lower_path.ends_with(EXTENSION_DOCS) => {
                        let data =
                            toml::de::from_str::<DocsFile>(&utf8str(path, io.read_file(path)?)?)
                                .into_diagnostic()
                                .context("failed to deserialize docs TOML")?;

                        docs.add_file(data, path.to_path_buf())?;
                    }
                    "toml"
                        if path != "project.toml"
                            && path.starts_with(&config.types_config.root) =>
                    {
                        let data =
                            toml::de::from_str::<JsonValue>(&utf8str(path, io.read_file(path)?)?)
                                .into_diagnostic()
                                .context("failed to deserialize types config TOML")?;
                        types_jsons.insert(path.to_path_buf(), data);
                    }
                    _ => {}
                }

                Ok(())
            })
            .with_context(|| format!("failed to load file at `{}`", path))?;
        }

        io.flush()?;

        let registry = ETypesRegistry::from_raws(registry_items, config)?;

        let mut project = Self {
            registry,
            docs,
            diagnostics: Default::default(),
            files: Default::default(),
            graphs: Default::default(),
            to_delete: Default::default(),
            history: UndoHistory::new(UndoSettings::default()),
            root,
            io,
        };

        project.validate_config()?;

        for (path, json) in types_jsons {
            let JsonValue::Object(obj) = json else {
                bail!(
                    "Type configuration should be an object, but instead got {}, in {}",
                    json_kind(&json),
                    path
                );
            };

            for (key, value) in obj {
                let cfg = project.registry.extra_config_mut(key);
                cfg.push((path.clone(), value));
            }
        }

        for (path, (json, ty)) in import_jsons {
            let item = match project
                .deserialize_json(json, ty)
                .with_context(|| format!("failed to deserialize JSON at `{}`", path))
            {
                Ok(data) => {
                    validate(
                        &project.registry,
                        project.diagnostics.enter(path.as_str()),
                        None,
                        &data,
                    )?;
                    if project.io.file_exists(generated_marker_path(&path))? {
                        ProjectFile::GeneratedValue(data)
                    } else {
                        ProjectFile::Value(data)
                    }
                }
                Err(err) => ProjectFile::BadValue(err),
            };
            project.files.insert(path, item);
        }

        for (path, mut json) in graphs {
            let graph = ProjectGraph::parse_json(&project.registry, &mut json)
                .with_context(|| format!("failed to deserialize Graph at `{}`", path))?;
            let file = project
                .graphs
                .add_graph(path.clone(), graph)
                .with_context(|| format!("failed to process Graph at `{}`", path))?;
            project.files.insert(path, file);
        }

        // Validate again after all files are loaded
        project.validate_all()?;

        Ok(project)
    }

    pub fn delete_file(&mut self, path: impl AsRef<Utf8Path>) -> miette::Result<()> {
        let path = path.as_ref();
        if let Some(removed) = self.files.remove(path) {
            if removed.is_generated() {
                self.to_delete.insert(generated_marker_path(path));
            }
            self.to_delete.insert(path.to_owned());
        }

        Ok(())
    }

    pub fn evaluate_graphs(&mut self) -> miette::Result<()> {
        let mut side_effects = side_effects::SideEffects::new();
        let mut generated = vec![];

        for graph in self.graphs.graphs.values_mut() {
            graph.graph_mut().ensure_region_graph_ready();
        }

        for (path, file) in &self.files {
            m_try(|| {
                if file.is_generated() {
                    generated.push(path.clone());
                    return Ok(());
                }
                let ProjectFile::Graph(id) = file else {
                    return Ok(());
                };

                let Some(graph) = self.graphs.graphs.get(id) else {
                    bail!("graph {:?} at path {} is not found", id, path);
                };

                if graph.is_node_group {
                    return Ok(());
                }
                let out_values = &mut None;
                let mut ctx = GraphExecutionContext::from_graph(
                    graph.graph(),
                    &self.registry,
                    Some(&self.graphs),
                    SideEffectsContext::new(&mut side_effects, path.clone(), &self.files),
                    graph.is_node_group,
                    &[],
                    out_values,
                );
                ctx.full_eval(true)?;
                drop(ctx);
                if out_values.is_some() {
                    bail!("graph {:?} at path {} has outputs", id, path);
                }

                Ok(())
            })
            .with_context(|| format!("failed to evaluate graph at `{}`", path))?
        }

        for path in generated {
            self.delete_file(&path)?;
        }

        side_effects.execute(self)?;

        Ok(())
    }

    /// Clean and validate the project, evaluating all graphs and running side effects
    pub fn clean_validate(&mut self) -> miette::Result<()> {
        self.diagnostics.diagnostics.clear();
        let graph_eval_time = Instant::now();
        self.evaluate_graphs()?;
        let graph_eval_time = graph_eval_time.elapsed().as_secs_f32();
        clear_validation_cache(&self.registry);
        let validate_time = Instant::now();
        // Double validate to ensure that validation cache is populated
        self.validate_all()?;
        self.validate_all()?;
        let validate_time = validate_time.elapsed().as_secs_f32();
        info!(
            graph_eval_time,
            validate_time, "Project built and validated successfully"
        );
        Ok(())
    }

    pub fn validate_all(&mut self) -> miette::Result<()> {
        for (path, file) in &self.files {
            match file {
                ProjectFile::Value(file) | ProjectFile::GeneratedValue(file) => {
                    validate(
                        &self.registry,
                        self.diagnostics.enter(path.as_str()),
                        None,
                        file,
                    )?;
                }
                ProjectFile::BadValue(_) => {
                    let mut ctx = self.diagnostics.enter(path.as_str());
                    ctx.clear_downstream();
                    ctx
                        .emit_error(miette!("failed to deserialize JSON at `{path}`, open the file in editor for details"));
                }
                &ProjectFile::Graph(_) => {
                    // TODO: validate graph
                }
            }
        }
        Ok(())
    }

    pub fn save(&mut self) -> miette::Result<()> {
        self.clean_validate()?;

        if self.diagnostics.has_diagnostics(DiagnosticLevel::Error) {
            return Err(miette!("project has unresolved errors, cannot save"));
        }

        let (no_delete_sender, no_delete_receiver) = std::sync::mpsc::channel::<Utf8PathBuf>();

        self.files.par_iter().try_for_each_with(
            no_delete_sender,
            |sender, (path, file)| -> miette::Result<()> {
                sender.send(path.clone()).unwrap();
                let mut generated = false;
                fn wrap_if_dbe(path: &Utf8Path, value: &EValue, json: JsonValue) -> JsonValue {
                    if path
                        .extension()
                        .is_some_and(|ext| ext.to_lowercase().ends_with(EXTENSION_VALUE))
                    {
                        let json = MiscJson {
                            ty: value.ty(),
                            value: json,
                        };

                        serde_json::value::to_value(&json)
                            .expect("serialization of MiscJson should not fail")
                    } else {
                        json
                    }
                }
                let json_string = m_try(|| {
                    let json = match file {
                        ProjectFile::Value(value) => {
                            wrap_if_dbe(path, value, self.serialize_json(value)?)
                        }
                        ProjectFile::GeneratedValue(value) => {
                            generated = true;
                            wrap_if_dbe(path, value, self.serialize_json(value)?)
                        }
                        ProjectFile::Graph(id) => {
                            let Some(graph) = self.graphs.graphs.get(id) else {
                                panic!("graph {:?} at path {} is not found", id, path);
                            };
                            graph.write_json(&self.registry)?
                        }
                        ProjectFile::BadValue(_) => {
                            panic!("BadValue should have been filtered out by validate_all");
                        }
                    };

                    let mut buf = vec![];
                    let mut serializer = serde_json::ser::Serializer::with_formatter(
                        &mut buf,
                        DBEJsonFormatter::pretty(),
                    );

                    json.serialize(&mut serializer).into_diagnostic()?;

                    Ok(String::from_utf8(buf).expect("JSON should be UTF-8"))
                })
                .with_context(|| format!("failed to serialize file at `{}`", path))?;

                if generated {
                    let generated_path = generated_marker_path(path);
                    sender.send(generated_path.clone()).unwrap();
                    self.io.write_file(&generated_path, &[]).with_context(|| {
                        format!("failed to write generated marker to `{}`", generated_path)
                    })?;
                }

                self.io
                    .write_file(path, json_string.as_bytes())
                    .with_context(|| format!("failed to write JSON to `{}`", path))?;

                Ok(())
            },
        )?;

        for p in no_delete_receiver.try_iter() {
            self.to_delete.remove(&p);
        }

        self.to_delete
            .par_drain()
            .try_for_each(|path| -> miette::Result<()> {
                self.io
                    .delete_file(&path)
                    .with_context(|| format!("failed to delete `{}`", path))?;

                Ok(())
            })?;

        self.io.flush()?;

        Ok(())
    }
}

impl<IO> Project<IO> {
    /// See [UndoHistory::undo]
    pub fn undo(&mut self) -> miette::Result<Utf8PathBuf> {
        self.history.undo(&mut self.files, &mut self.graphs)
    }

    /// See [UndoHistory::redo]
    pub fn redo(&mut self) -> miette::Result<Utf8PathBuf> {
        self.history.redo(&mut self.files, &mut self.graphs)
    }

    /// See [UndoHistory::check_file]
    pub fn file_changed(&mut self, path: &Utf8PathBuf, force_snapshot: bool) -> miette::Result<()> {
        self.history
            .check_file(&self.files, &self.graphs, path, force_snapshot)
    }

    pub fn import_root(&self) -> EDataType {
        EDataType::Object {
            ident: self.registry.project_config().types_config.import,
        }
    }
}

fn generated_marker_path(file: impl AsRef<Utf8Path>) -> Utf8PathBuf {
    let file = file.as_ref();
    file.parent()
        .expect("Path has parent")
        .join(file.file_name().expect("Path has file name").to_string() + ".generated")
}

impl<IO: ProjectIO> Project<IO> {
    // pub fn get_value(&mut self, path: &Utf8Path) -> Option<&mut ProjectFile> {
    //     self.files.get_mut(path)
    // }

    fn validate_config(&self) -> miette::Result<()> {
        self.registry
            .get_object(&self.registry.project_config().types_config.import)
            .ok_or_else(|| {
                miette!(
                    "unknown type `{}`",
                    self.registry.project_config().types_config.import
                )
            })
            .context("failed to validate [types.import] config entry")
            .context("project config is invalid")?;

        Ok(())
    }

    fn deserialize_json(
        &self,
        mut value: JsonValue,
        ty: Option<EDataType>,
    ) -> miette::Result<EValue> {
        let ty = ty.unwrap_or_else(|| EDataType::Object {
            ident: self.registry.project_config().types_config.import,
        });

        ty.parse_json(&self.registry, &mut value, false)
    }

    fn serialize_json(&self, value: &EValue) -> miette::Result<JsonValue> {
        // let object = self
        //     .registry
        //     .get_object(&self.config.types_config.import)
        //     .expect("Config was validated");

        // object.parse_json(&self.registry, &mut value, false)
        value.write_json(&self.registry)
    }
}
