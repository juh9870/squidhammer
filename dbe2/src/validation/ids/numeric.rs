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
use miette::{bail, miette, Diagnostic};
use parking_lot::RwLock;
use serde::Deserialize;
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
#[error("Duplicate ID of type {}", .ty)]
struct DuplicateIdError {
    ty: Ustr,
    others: Vec<String>,
}

impl Diagnostic for DuplicateIdError {
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(format!(
            "duplicate IDs in:\n\t{}",
            self.others.join("\n\t")
        )))
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("ID {} of type {} is reserved", .id, .ty)]
struct ReservedIdError {
    ty: Ustr,
    id: ENumber,
}

#[derive(Debug, Default, Deserialize)]
struct ReservedIdConfig {
    reserved_ids: UstrMap<AHashSet<ENumber>>,
}

impl ConfigMerge for ReservedIdConfig {
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
        )
    }
}

fn reserved_ids(reg: &ETypesRegistry, ty: Ustr) -> miette::Result<Arc<ReservedIdConfig>> {
    reg.config().get::<ReservedIdConfig>("ids/numeric")
}

#[derive(Debug)]
pub struct Id;

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

        let (ty, id) = ty_and_id(registry, data)?;

        let reserved = reserved_ids(registry, ty)?;

        if reserved
            .reserved_ids
            .get(&ty)
            .is_some_and(|ids| ids.contains(&id))
        {
            ctx.emit_error(ReservedIdError { ty, id }.into());
            return Ok(());
        }

        let ids = reg.ids.entry(ty).or_default().entry(id).or_default();

        // trace!("validating id: `{}` for type `{:?}`", id, ty);

        let path = ctx.ident();
        ids.insert(path.to_string());

        if ids.len() > 1 {
            ctx.emit_error(
                DuplicateIdError {
                    ty,
                    others: ids.iter().filter(|other| *other != path).cloned().collect(),
                }
                .into(),
            );
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

        let ids = reg.ids.entry(ty).or_default().entry(id).or_default();

        if ids.is_empty() {
            let reserved = reserved_ids(registry, ty)?;

            if !reserved
                .reserved_ids
                .get(&ty)
                .is_some_and(|ids| ids.contains(&id))
            {
                ctx.emit_error(miette!("ID {} of type {} is not defined", id, ty));
            }
        }

        Ok(())
    }
}
