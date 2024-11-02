use crate::etype::eitem::EItemInfo;
use crate::registry::ETypesRegistry;
use ahash::AHashMap;
use diagnostic::context::DiagnosticContextMut;
use itertools::Itertools;
use miette::{bail, miette, Diagnostic};
use parking_lot::RwLock;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Display;
use thiserror::Error;
use tracing::trace;
use ustr::{Ustr, UstrMap};

use crate::validation::DataValidator;
use crate::value::{ENumber, EValue};

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
        .generic_arguments
        .iter()
        .exactly_one()
        .map_err(|_| {
            miette!(
                "expected struct with exactly one generic argument, got {:?}",
                obj_data.generic_arguments.len()
            )
        })?;

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

    Ok((*arg, *id))
}

#[derive(Debug, Error)]
#[error("Duplicate ID")]
struct DuplicateIdError {
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

        let ids = reg.ids.entry(ty).or_default().entry(id).or_default();

        trace!("validating id: {:?}", id);

        let path = ctx.ident();
        ids.insert(path.to_string());

        if ids.len() > 1 {
            ctx.emit_error(
                DuplicateIdError {
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
            ctx.emit_error(miette!("ID `{}` is not defined", id));
        }

        Ok(())
    }
}
