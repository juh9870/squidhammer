use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::registry::config::merge::ConfigMerge;
use crate::registry::ETypesRegistry;
use crate::validation::DataValidator;
use crate::value::id::ETypeId;
use crate::value::{ENumber, EValue};
use camino::Utf8PathBuf;
use diagnostic::context::DiagnosticContextMut;
use itertools::Itertools;
use miette::{bail, miette, Context, Diagnostic};
use parking_lot::RwLock;
use parking_lot::RwLockWriteGuard;
use serde::Deserialize;
use smallvec::{smallvec, SmallVec};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::sync::Arc;
use thiserror::Error;
use ustr::{Ustr, UstrMap};
use utils::map::{HashMap, HashSet};

#[derive(Default)]
pub struct NumericIDsRegistry {
    ids: UstrMap<HashMap<ENumber, BTreeSet<String>>>,
}

type Data = RwLock<NumericIDsRegistry>;

/// Extracts the struct type and ID from an ID struct value
fn ty_and_id(registry: &ETypesRegistry, data: &EValue) -> miette::Result<(Ustr, ENumber)> {
    let EValue::Struct { ident, fields } = data else {
        bail!("expected an ID struct value, got {:?}", data);
    };

    let arg = extract_generic_arg(registry, ident)?;

    let Some(id) = fields.get(&Ustr::from("id")) else {
        bail!(
            "expected field `id` in an ID struct, got {:?}",
            fields.keys().map(|x| x.as_str()).join(", ")
        );
    };

    let id = id.try_as_number().map_err(|_| {
        miette!(
            "expected numeric value for `id` field in an ID struct, got {:?}",
            id
        )
    })?;

    Ok((arg, *id))
}

fn extract_generic_arg(registry: &ETypesRegistry, ident: &ETypeId) -> miette::Result<Ustr> {
    let obj_data = registry
        .get_struct(ident)
        .ok_or_else(|| miette!("unknown object type or not a struct: `{:?}`", ident))?;

    let arg = obj_data
        .generic_arguments_values
        .iter()
        .exactly_one()
        .map_err(|_| {
            miette!(
                "expected struct with exactly one generic argument called `Id`, got {:?} arguments",
                obj_data.generic_arguments.len()
            )
        })?
        .ty();

    if obj_data.generic_arguments[0] != "Id" {
        bail!(
            "expected generic argument to be called `Id`, got `{}`",
            obj_data.generic_arguments[0]
        );
    }

    let EDataType::Const { value } = arg else {
        bail!(
            "generic argument `Id` is expected to be a constant string, got `{:?}`",
            arg
        );
    };

    let Some(arg) = value.as_string() else {
        bail!(
            "generic argument `Id` is expected to be a constant string, got constant `{:?}`",
            value
        );
    };

    Ok(arg)
}

#[derive(Debug, Error)]
enum IdValidationError {
    #[error("duplicate ID of type {}", .ty)]
    DuplicateId { ty: Ustr, others: Vec<String> },
    #[error("ID {} of type {} is reserved", .id, .ty)]
    ReservedId { ty: Ustr, id: ENumber },
    #[error("ID of type {} conflicts with ID of type {}", .ty, .conflicting)]
    FromConflicting {
        ty: Ustr,
        conflicting: Ustr,
        #[source]
        error: Box<IdValidationError>,
    },
}

impl Diagnostic for IdValidationError {
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        match self {
            IdValidationError::DuplicateId { ty: _, others } => Some(Box::new(format!(
                "duplicate IDs in:\n\t{}",
                others.join("\n\t")
            ))),
            _ => None,
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        match self {
            IdValidationError::FromConflicting { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ReservedIdConfig {
    types: UstrMap<ReservedIdTypeConfig>,
}

impl ConfigMerge for ReservedIdConfig {
    fn merge(
        &mut self,
        paths: &[&Utf8PathBuf],
        other: Self,
        other_path: &Utf8PathBuf,
    ) -> miette::Result<()> {
        ConfigMerge::merge(&mut self.types, paths, other.types, other_path)
    }
}

#[derive(Debug, Default, Deserialize)]
struct ReservedIdTypeConfig {
    /// Reserved IDs
    #[serde(default)]
    reserved_ids: HashSet<ENumber>,
    /// Other types that this type cannot share IDs with
    #[serde(default)]
    conflicting_types: Vec<Ustr>,
    /// Other types that this ID can be satisfied by
    #[serde(default)]
    satisfied_by_types: Vec<Ustr>,
}

impl ConfigMerge for ReservedIdTypeConfig {
    fn merge(
        &mut self,
        paths: &[&Utf8PathBuf],
        other: Self,
        other_path: &Utf8PathBuf,
    ) -> miette::Result<()> {
        ConfigMerge::merge(
            &mut self.reserved_ids,
            paths,
            other.reserved_ids,
            other_path,
        )?;
        ConfigMerge::merge(
            &mut self.conflicting_types,
            paths,
            other.conflicting_types,
            other_path,
        )?;
        ConfigMerge::merge(
            &mut self.satisfied_by_types,
            paths,
            other.satisfied_by_types,
            other_path,
        )?;
        Ok(())
    }
}

fn config(reg: &ETypesRegistry) -> miette::Result<Arc<ReservedIdConfig>> {
    reg.config().get::<ReservedIdConfig>("ids/numeric")
}

#[derive(Debug, Copy, Clone)]
pub struct NumericIDRegistry<'a> {
    registry: &'a ETypesRegistry,
}

impl<'a> NumericIDRegistry<'a> {
    pub fn of(registry: &'a ETypesRegistry) -> Self {
        Self { registry }
    }

    pub fn is_id_assignable(&self, from: Ustr, to: Ustr) -> miette::Result<bool> {
        fn is_assignable(
            config: &ReservedIdConfig,
            from: Ustr,
            to: Ustr,
            visited: &mut SmallVec<[Ustr; 2]>,
        ) -> bool {
            if visited.contains(&to) {
                return false;
            }
            visited.push(to);

            if from == to {
                return true;
            }

            if let Some(cfg) = config.types.get(&to) {
                for ty in &cfg.satisfied_by_types {
                    if is_assignable(config, from, *ty, visited) {
                        return true;
                    }
                }
            }

            false
        }

        let config = config(self.registry)?;

        Ok(is_assignable(&config, from, to, &mut smallvec![]))
    }

    /// Checks if the given types are assignable, i.e. if an ID of type `from`
    /// can be assigned to a reference of type `to`
    pub fn is_id_assignable_ty(&self, from: EDataType, to: EDataType) -> miette::Result<bool> {
        let (EDataType::Object { ident: from }, EDataType::Object { ident: to }) = (from, to)
        else {
            bail!("expected object types, got {:?} and {:?}", from, to);
        };

        let from_arg = extract_generic_arg(self.registry, &from)?;
        let to_arg = extract_generic_arg(self.registry, &to)?;

        self.is_id_assignable(from_arg, to_arg)
    }

    /// Returns the location of the given ID, if it exists
    ///
    /// Exact format of the location is not specified, but it should be
    /// human-readable
    pub fn location_for_id(
        &self,
        ref_ty: EDataType,
        id: ENumber,
    ) -> miette::Result<Option<String>> {
        fn location(
            config: &ReservedIdConfig,
            reg: &NumericIDsRegistry,
            id: ENumber,
            category: Ustr,
            visited: &mut SmallVec<[Ustr; 2]>,
        ) -> Option<String> {
            if visited.contains(&category) {
                return None;
            }
            visited.push(category);

            if let Some(id) = reg
                .ids
                .get(&category)
                .and_then(|m| m.get(&id))
                .and_then(|s| s.iter().next())
            {
                return Some(id.clone());
            };

            if let Some(cfg) = config.types.get(&category) {
                for ty in &cfg.satisfied_by_types {
                    if let Some(id) = location(config, reg, id, *ty, visited) {
                        return Some(id);
                    }
                }
            }

            None
        }

        let EDataType::Object { ident } = ref_ty else {
            bail!("expected object type, got {:?}", ref_ty);
        };

        let category = extract_generic_arg(self.registry, &ident)?;

        let config = config(self.registry)?;

        let reg = self.registry.extra_data::<Data>();
        let reg = reg.read();

        Ok(location(&config, &reg, id, category, &mut smallvec![]))
    }

    /// Runs the provided closure with an iterator over available IDs for the
    /// given type, as well as the reserved IDs, and all IDs for types that
    /// this type is satisfied by
    pub fn with_available_ids<T>(
        &self,
        ref_ty: EDataType,
        cb: impl FnOnce(AvailableIdsIter) -> miette::Result<T>,
    ) -> miette::Result<T> {
        let EDataType::Object { ident } = ref_ty else {
            bail!("expected object type, got {:?}", ref_ty);
        };

        let category = extract_generic_arg(self.registry, &ident)?;

        let config = config(self.registry)?;

        let reg = self.registry.extra_data::<Data>();
        let reg = reg.read();

        let iter = AvailableIdsIter::new(&reg, &config, smallvec![category]);

        cb(iter)
    }
}

/// Iterator over available IDs for a given type
///
/// This iterator will yield all available IDs for a given type, as well as the
/// reserved IDs for that type, and the same for all `satisfied_by` types
pub struct AvailableIdsIter<'a> {
    reg: &'a NumericIDsRegistry,
    cfg: &'a ReservedIdConfig,
    categories: SmallVec<[Ustr; 1]>,
    cur_iter: Option<std::collections::hash_map::Iter<'a, ENumber, BTreeSet<String>>>,
    cur_reserved_iter: Option<std::collections::hash_set::Iter<'a, ENumber>>,
}

impl<'a> AvailableIdsIter<'a> {
    fn new(
        reg: &'a NumericIDsRegistry,
        cfg: &'a ReservedIdConfig,
        categories: SmallVec<[Ustr; 1]>,
    ) -> Self {
        Self {
            reg,
            cfg,
            categories,
            cur_iter: None,
            cur_reserved_iter: None,
        }
    }
}

impl<'a> Iterator for AvailableIdsIter<'a> {
    type Item = (&'a ENumber, Option<&'a BTreeSet<String>>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.cur_iter {
            if let Some(next) = iter.next() {
                return Some((next.0, Some(next.1)));
            }
            {
                self.cur_iter = None;
            }
        } else if let Some(iter) = &mut self.cur_reserved_iter {
            if let Some(next) = iter.next() {
                return Some((next, None));
            } else {
                self.cur_reserved_iter = None;
            }
        }

        // cur iter is exhausted, get the next one

        while let Some(category) = self.categories.pop() {
            let reserved_ids = if let Some(config) = self.cfg.types.get(&category) {
                self.categories
                    .extend(config.satisfied_by_types.iter().cloned());
                (!config.reserved_ids.is_empty()).then(|| config.reserved_ids.iter())
            } else {
                None
            };

            let ids = self
                .reg
                .ids
                .get(&category)
                .filter(|ids| !ids.is_empty())
                .map(|ids| ids.iter());

            if reserved_ids.is_none() && ids.is_none() {
                continue;
            }

            self.cur_iter = ids;
            self.cur_reserved_iter = reserved_ids;
            return self.next();
        }

        // all categories are exhausted, end of iteration
        None
    }
}

#[derive(Debug)]
pub struct Id;

impl DataValidator for Id {
    fn name(&self) -> Cow<'static, str> {
        "ids/numeric".into()
    }

    fn clear_cache(&self, registry: &ETypesRegistry) {
        let ids = registry.extra_data::<Data>();
        let mut ids = ids.write();
        ids.ids.clear();
    }

    fn validate(
        &self,
        registry: &ETypesRegistry,
        mut ctx: DiagnosticContextMut,
        _item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        fn check_id_conflicts(
            registry: &ETypesRegistry,
            reg: &mut RwLockWriteGuard<NumericIDsRegistry>,
            mut ctx: DiagnosticContextMut,
            ty: Ustr,
            id: ENumber,
            visited: &mut SmallVec<[Ustr; 2]>,
            top: bool,
        ) -> miette::Result<Vec<IdValidationError>> {
            if visited.contains(&ty) {
                return Ok(vec![]);
            }
            visited.push(ty);

            let mut errors = vec![];

            let config = config(registry)?;
            if let Some(config) = config.types.get(&ty) {
                if config.reserved_ids.contains(&id) {
                    errors.push(IdValidationError::ReservedId { ty, id });
                }
                for conflicting in &config.conflicting_types {
                    let conflicts = check_id_conflicts(
                        registry,
                        reg,
                        ctx.enter_inline(),
                        *conflicting,
                        id,
                        visited,
                        false,
                    )
                    .with_context(|| format!("in conflicting type `{}`", conflicting))?;
                    errors.extend(conflicts.into_iter().map(|error| {
                        IdValidationError::FromConflicting {
                            ty,
                            conflicting: *conflicting,
                            error: Box::new(error),
                        }
                    }));
                }
            }

            let ids = reg.ids.entry(ty).or_default().entry(id).or_default();

            // trace!("validating id: `{}` for type `{:?}`", id, ty);

            let mut filter_out_path = None;
            if top {
                let path = ctx.full_path();
                ids.insert(path.to_string());
                filter_out_path = Some(path);
            }

            if ids.len() > if top { 1 } else { 0 } {
                errors.push(IdValidationError::DuplicateId {
                    ty,
                    others: if let Some(path) = filter_out_path {
                        ids.iter()
                            .filter(|other| *other != &path)
                            .cloned()
                            .collect()
                    } else {
                        ids.iter().cloned().collect()
                    },
                });
            }
            Ok(errors)
        }

        let reg = registry.extra_data::<Data>();
        let mut reg = reg.write();

        let (ty, id) = ty_and_id(registry, data)?;

        let errors = check_id_conflicts(
            registry,
            &mut reg,
            ctx.enter_inline(),
            ty,
            id,
            &mut smallvec![],
            true,
        )?;

        for error in errors {
            ctx.emit_error(error.into());
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Ref;

impl DataValidator for Ref {
    fn name(&self) -> Cow<'static, str> {
        "ids/numeric_ref".into()
    }

    fn clear_cache(&self, _registry: &ETypesRegistry) {
        // cache is cleared by the `Id` validator
    }

    fn validate(
        &self,
        registry: &ETypesRegistry,
        mut ctx: DiagnosticContextMut,
        _item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        fn check_id_exists(
            registry: &ETypesRegistry,
            reg: &mut RwLockWriteGuard<NumericIDsRegistry>,
            ty: Ustr,
            id: ENumber,
            visited: &mut SmallVec<[Ustr; 2]>,
        ) -> miette::Result<bool> {
            if visited.contains(&ty) {
                return Ok(false);
            }
            visited.push(ty);

            let config = config(registry)?;
            if let Some(config) = config.types.get(&ty) {
                if config.reserved_ids.contains(&id) {
                    return Ok(true);
                }
                for satisfied in &config.satisfied_by_types {
                    if check_id_exists(registry, reg, *satisfied, id, visited)
                        .with_context(|| format!("in satisfied_by type `{}`", satisfied))?
                    {
                        return Ok(true);
                    }
                }
            }

            Ok(!reg
                .ids
                .entry(ty)
                .or_default()
                .entry(id)
                .or_default()
                .is_empty())
        }

        let reg = registry.extra_data::<Data>();
        let mut reg = reg.write();

        let (ty, id) = ty_and_id(registry, data)?;

        if !check_id_exists(registry, &mut reg, ty, id, &mut smallvec![])? {
            ctx.emit_error(miette!("ID {} of type `{}` is not defined", id, ty));
        }

        Ok(())
    }
}
