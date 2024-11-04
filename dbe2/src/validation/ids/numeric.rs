use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::registry::config::merge::ConfigMerge;
use crate::registry::ETypesRegistry;
use crate::validation::DataValidator;
use crate::value::{ENumber, EValue};
use ahash::{AHashMap, AHashSet};
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

#[derive(Default)]
pub struct NumericIDsRegistry {
    ids: UstrMap<AHashMap<ENumber, BTreeSet<String>>>,
}

type Data = RwLock<NumericIDsRegistry>;

/// Extracts the struct type and ID from an ID struct value
fn ty_and_id(registry: &ETypesRegistry, data: &EValue) -> miette::Result<(Ustr, ENumber)> {
    let EValue::Struct { ident, fields } = data else {
        bail!("expected an ID struct value, got {:?}", data);
    };

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
    reserved_ids: AHashSet<ENumber>,
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

#[derive(Debug)]
pub struct Id;

const MAX_DEPTH: usize = 16;

impl DataValidator for Id {
    fn name(&self) -> Cow<'static, str> {
        "ids/numeric".into()
    }

    fn validate(
        &self,
        registry: &ETypesRegistry,
        mut ctx: DiagnosticContextMut,
        _item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        let reg = registry.extra_data::<Data>();
        let mut reg = reg.write();

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

    fn validate(
        &self,
        registry: &ETypesRegistry,
        mut ctx: DiagnosticContextMut,
        _item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        let reg = registry.extra_data::<Data>();
        let mut reg = reg.write();

        let (ty, id) = ty_and_id(registry, data)?;

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

        if !check_id_exists(registry, &mut reg, ty, id, &mut smallvec![])? {
            ctx.emit_error(miette!("ID {} of type `{}` is not defined", id, ty));
        }

        Ok(())
    }
}
