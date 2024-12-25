use crate::etype::EDataType;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use ahash::{AHashMap, AHashSet};
use miette::{bail, miette, Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::hash_map::Entry;
use std::ops::RangeInclusive;
use std::sync::LazyLock;

pub static STORAGE_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:mappings/storage".into()));
pub static RANGE_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:math/range".into()));

#[derive(Debug, Clone, Default)]
pub struct Mappings {
    /// Mapping of the string id <-> numeric ID
    ids: AHashMap<String, MappingEntry>,
    /// List of string IDs that were "created" by the current session
    ///
    /// Mainly here to allow for proper functioning of the [Mappings::new_id],
    /// which checks whenever the ID was already "created" during the current
    /// session
    currently_created: AHashSet<String>,
    /// Set of all occupied IDs
    occupied_ids: AHashSet<i64>,
    /// List of available ID ranges
    ///
    /// This uses range iterators, that are consumed when an ID is taken. Due
    /// to this, type definition may need to be changed when rust Ranges are
    /// "fixed"
    available_ids: SmallVec<[RangeInclusive<i64>; 1]>,
    /// Default available ID ranges
    default_ranges: SmallVec<[PackedRange; 1]>,
}

#[derive(Debug, Clone)]
struct MappingEntry {
    id: i64,
    persistent: bool,
}

impl Mappings {
    /// Returns an ID corresponding to the given string ID, or establishing a
    /// new link
    pub fn get_id_raw(&mut self, id: String, persistent: bool) -> miette::Result<i64> {
        match self.ids.entry(id) {
            Entry::Occupied(entry) => Ok(entry.get().id),
            Entry::Vacant(entry) => {
                let numeric = next_id_raw(&mut self.available_ids, &mut self.occupied_ids)?;
                entry.insert(MappingEntry {
                    id: numeric,
                    persistent,
                });
                Ok(numeric)
            }
        }
    }

    /// Establishes a new link between the string ID and the numeric ID, or
    /// bails if the string ID is already taken
    pub fn new_id(&mut self, id: String, persistent: bool) -> miette::Result<i64> {
        if !self.currently_created.insert(id.clone()) {
            bail!("ID `{}` is already taken", id);
        }
        self.get_id_raw(id, persistent)
    }

    /// Returns the numeric ID corresponding to the given string ID if it exists
    pub fn existing_id(&self, id: &str) -> miette::Result<i64> {
        if !self.currently_created.contains(id) {
            bail!("ID `{}` is not yet created via `NewId` mapping", id);
        }
        self.ids
            .get(id)
            .map(|e| e.id)
            .ok_or_else(|| miette!("ID `{}` does not exist", id))
    }

    /// Returns whenever the mapping contains any persistent IDs
    pub fn has_persistent_ids(&self) -> bool {
        self.ids.iter().any(|(_, v)| v.persistent)
    }
}

impl Mappings {
    pub fn new(ranges: &EValue) -> miette::Result<Self> {
        let ranges = match ranges {
            EValue::List { values, .. } => values
                .iter()
                .map(|v| match v {
                    EValue::Struct { fields, ident } => {
                        if ident != &*RANGE_ID {
                            bail!("Expected a {} struct, got {:?}", *RANGE_ID, ident);
                        }

                        let start = fields
                            .get(&"start".into())
                            .and_then(|v| v.try_as_number().ok())
                            .ok_or_else(|| miette!("Expected a number in `start` field"))?;

                        let end = fields
                            .get(&"end".into())
                            .and_then(|v| v.try_as_number().ok())
                            .ok_or_else(|| miette!("Expected a number in `end` field"))?;

                        Ok(PackedRange {
                            start: start.0.trunc(),
                            end: end.0.trunc(),
                        })
                    }
                    _ => bail!("Expected a struct, got {:?}", v),
                })
                .collect::<miette::Result<SmallVec<[PackedRange; 1]>>>()?,
            _ => bail!("Expected a list of ranges, got {:?}", ranges),
        };

        Ok(Self {
            ids: Default::default(),
            currently_created: Default::default(),
            occupied_ids: Default::default(),
            available_ids: ranges
                .iter()
                .map(|r| (r.start as i64)..=(r.end as i64))
                .collect(),
            default_ranges: ranges,
        })
    }

    pub fn from_evalue(registry: &ETypesRegistry, value: &EValue) -> miette::Result<Self> {
        let EValue::Struct { fields: _, ident } = &value else {
            bail!("Expected a struct, got {:?}", value);
        };

        if ident != &*STORAGE_ID {
            bail!("Expected a {} struct, got {:?}", *STORAGE_ID, ident);
        }

        // TODO: skip json step and directly parse evalue? Make evalue serde-able?

        let json = value.write_json(registry)?;

        let mappings: PackedMappings = serde_json::from_value(json)
            .into_diagnostic()
            .context("failed to load mappings from serialized value")?;

        // debug!("Loaded packed mappings: {:?}", mappings);

        Ok(mappings.into_mappings())
    }

    pub fn as_evalue(&self, registry: &ETypesRegistry) -> miette::Result<EValue> {
        let mappings = PackedMappings::from_mappings(self);

        let mut json = serde_json::to_value(mappings)
            .into_diagnostic()
            .context("failed to serialize mappings")?;

        // debug!("Serialized packed mappings: {:?}", json);

        EDataType::Object { ident: *STORAGE_ID }.parse_json(registry, &mut json, false)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackedMappings {
    values: AHashMap<String, f64>,
    ranges: SmallVec<[PackedRange; 1]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackedRange {
    start: f64,
    end: f64,
}

impl PackedMappings {
    pub fn into_mappings(self) -> Mappings {
        let m = Mappings {
            occupied_ids: self.values.values().map(|v| *v as i64).collect(),
            ids: self
                .values
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        MappingEntry {
                            id: v as i64,
                            persistent: true,
                        },
                    )
                })
                .collect(),
            available_ids: self
                .ranges
                .iter()
                .map(|r| (r.start as i64)..=(r.end as i64))
                .collect(),
            default_ranges: self.ranges,
            currently_created: Default::default(),
        };

        // debug!("Converted packed mappings to full: {:?}", m);

        m
    }

    pub fn from_mappings(mappings: &Mappings) -> Self {
        // debug!("Preparing to convert mappings to packed: {:?}", mappings);
        let p = PackedMappings {
            values: mappings
                .ids
                .iter()
                .filter_map(|(k, v)| v.persistent.then_some((k.clone(), v.id as f64)))
                .collect(),
            ranges: mappings.default_ranges.clone(),
        };

        // debug!("Converted mappings to packed: {:?}", p);

        p
    }
}

fn next_id_raw(
    available_ids: &mut [RangeInclusive<i64>],
    occupied_ids: &mut AHashSet<i64>,
) -> miette::Result<i64> {
    let ids = available_ids;

    if ids.is_empty() {
        bail!("No ID ranges are available. Please add a new range to the available IDs")
    }

    while let Some(id) = ids.iter_mut().find_map(|range| range.next()) {
        // Check that ID is not already occupied
        if !occupied_ids.contains(&id) {
            occupied_ids.insert(id);
            return Ok(id);
        }
    }

    bail!("No free IDs are left");
}
