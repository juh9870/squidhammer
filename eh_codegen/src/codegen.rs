use camino::{Utf8Path, Utf8PathBuf};
use codegen_schema::schema::{
    SchemaData, SchemaDataType, SchemaEnumItem, SchemaStructMember, SchemaStructMemberType,
};
use convert_case::{Case, Casing};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;
use tracing::error_span;
use utils::map::{HashMap, HashSet};

static SPECIAL_FIELDS: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    let entries = [
        (
            "eh:objects/component_data::AmmunitionId$0",
            "object \"AmmunitionId\" \"sys:optional\" alias=\"Ammunition\" {\n\t\tobject \"Item\" \"sys:ids/numeric_ref\" {\n\t\t\tconst \"Id\" \"eh:ids/any_ammunition\"\n\t\t}\n\t}",
        ),
        ("eh:objects/component_data::AmmunitionId$1", ""),
        ("eh:objects/weapon/bullet_trigger_spawn_bullet::Color$0", "object \"Color\" default=\"#00000000\" \"color:argb\""),
        ("eh:objects/weapon/bullet_trigger_play_sfx::Color$0", "object \"Color\" default=\"#00000000\" \"color:argb\""),
        ("eh:objects/weapon/bullet_trigger_spawn_static_sfx::Color$0", "object \"Color\" default=\"#00000000\" \"color:argb\""),
        ("eh:objects/weapon/visual_effect_element::Color$0", "object \"Color\" default=\"#00000000\" \"color:argb\""),
    ];

    entries.iter().copied().collect()
});

static NO_DEPRECATION: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    let entries = [
        "eh:objects/faction_data::Hidden$0",
        "eh:objects/faction_data::Hostile$0",
        "eh:objects/drone_bay_data::ImprovedAi$0",
        "eh:objects/component_data::CellType$0",
        "eh:objects/component_data::WeaponSlotType$0",
    ];

    entries.iter().copied().collect()
});

#[derive(Debug, Default)]
pub struct Ctx {
    files: BTreeMap<String, String>,
    enums: BTreeMap<String, EnumData>,
    structs: BTreeMap<String, SchemaData>,
    typeids: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct EnumData {
    pub name: String,
    pub id: String,
    pub item: Vec<SchemaEnumItem>,
    pub linked_struct: Option<(Vec<String>, String)>,
}

impl Ctx {
    pub fn finish(mut self, output: &Path) {
        for (path, data) in std::mem::take(&mut self.structs).into_iter() {
            self.process_struct(path.into(), data);
        }

        for data in std::mem::take(&mut self.enums).into_values() {
            self.emit_enum(data);
        }

        for (id, content) in self.files {
            let path = Utf8PathBuf::from(id.strip_prefix("eh:").unwrap().to_string() + ".kdl");
            let path = output.join(path);
            let parent = path.parent().expect("All paths should have parents");

            fs_err::create_dir_all(parent).expect("Failed to create parent directory");
            fs_err::write(path, content).expect("Failed to write file");
        }
    }

    pub fn consume_enum(&mut self, path: Utf8PathBuf, data: SchemaData) {
        let _guard = error_span!("Enum", name = &data.name).entered();

        self.typeids.insert(data.name.clone(), id(&path));

        let item = data.item.expect("Should have enum items");

        if let Some(e) = self.enums.insert(
            data.name.clone(),
            EnumData {
                name: data.name,
                id: id(&path),
                item,
                linked_struct: None,
            },
        ) {
            panic!("Duplicate enum definition for `{}`", e.name)
        }
    }

    pub fn consume_struct(&mut self, path: Utf8PathBuf, data: SchemaData) {
        self.typeids.insert(data.name.clone(), id(&path));
        self.structs.insert(path.to_string(), data);
    }

    fn process_struct(&mut self, path: Utf8PathBuf, data: SchemaData) {
        let _guard = error_span!("Struct", name = &data.name).entered();

        if let Some(_ty) = &data.typeid {
            let name = data.name.clone();
            if data.ty != SchemaDataType::Settings {
                let data_path = Utf8PathBuf::from(path.to_string() + "Data");
                let data_id = self.consume_struct_raw(data_path.clone(), data, true);
                self.output(
                    id(&path),
                    format!(
                        "struct title=\"{}\" {{\n\tobject \"Id\" \"sys:ids/numeric\" {{\n\t\tconst \"Id\" \"{}\"\n\t}}\n\tobject \"Data\" \"{}\" inline=true\n}}",
                        &name,
                        id(&path),
                        data_id,
                    ),
                );
            } else {
                self.consume_struct_raw(path, data, false);
            }
        } else {
            self.consume_struct_raw(path, data, false);
        }
    }

    fn consume_struct_raw(
        &mut self,
        path: Utf8PathBuf,
        data: SchemaData,
        is_switch_variant: bool,
    ) -> String {
        let members = data.member.unwrap_or_default();
        if let Some(switch_field) = data.switch {
            let enum_name = members
                .iter()
                .find(|i| i.name == switch_field)
                .expect("Switch field should be in members")
                .typeid
                .clone()
                .expect("Switch field should have a typeid");

            let mut edata = self
                .enums
                .get(&enum_name)
                .expect("Should have linked enum at this point")
                .clone();

            let parent = path.parent().expect("All paths should have parents");

            let variants = edata
                .item
                .iter()
                .map(|i| id(&parent.join(format!("{}{}", data.name, i.name))))
                .collect_vec();

            if edata
                .linked_struct
                .replace((variants.clone(), switch_field.clone()))
                .is_some()
            {
                panic!("Enum can only have one linked struct")
            }

            let enum_id = id(&path);
            edata.id = enum_id.clone();
            edata.name = data.name;
            if is_switch_variant {
                edata.name = format!("{}Data", edata.name);
            }
            for (variant, variant_name) in variants
                .iter()
                .zip(edata.item.iter().map(|i| i.name.clone()).collect_vec())
            {
                let combined = format!("{}{}", &edata.name, variant_name);
                self.emit_struct(
                    variant.clone(),
                    &combined,
                    members
                        .iter()
                        .filter(|m| {
                            m.name != switch_field
                                && !m.case.as_ref().is_some_and(|c| {
                                    !c.split(",").any(|c| c.trim() == variant_name)
                                })
                        })
                        .cloned()
                        .collect_vec(),
                    false,
                );
            }
            self.enums.insert(edata.name.clone(), edata);
            enum_id
        } else {
            let id = id(&path);
            let name = if is_switch_variant {
                format!("{}Data", data.name)
            } else {
                data.name.clone()
            };
            self.emit_struct(
                id.clone(),
                &name,
                members,
                data.ty == SchemaDataType::Settings,
            );
            id
        }
    }

    fn emit_struct(
        &mut self,
        id: String,
        title: &str,
        members: Vec<SchemaStructMember>,
        singleton: bool,
    ) {
        let _guard = error_span!("Struct", id = &id).entered();
        let mut code = if singleton {
            format!("struct title=\"{}\" singleton=true {{", title)
        } else {
            format!("struct title=\"{}\" {{", title)
        };

        let typeid_fmt = |id: String| format!("\"{}\"", typeid(&self.typeids, id,));

        let mut encountered = HashMap::default();

        for member in members {
            let _guard = error_span!("Member", name = member.name).entered();

            let times_encountered = encountered.entry(member.name.clone()).or_insert(0);
            let field_path = format!("{}::{}${}", id, member.name, times_encountered);
            *times_encountered += 1;

            if member.name == "AmmunitionId" {
                dbg!(&field_path);
            }

            if let Some(special) = SPECIAL_FIELDS.get(&field_path.as_str()) {
                if !special.is_empty() {
                    code += &format!("\n\t{}", special.trim());
                }
                continue;
            }

            if member
                .options
                .as_ref()
                .is_some_and(|opts| opts.contains("obsolete"))
                && !NO_DEPRECATION.contains(&field_path.as_str())
            {
                continue;
            }

            let mut args = vec![];
            let mut generics = vec![];

            if let Some(alias) = &member.alias {
                args.push(format!("alias=\"{}\"", alias));
            }
            if let Some(default) = &member.default {
                let default = match member.ty {
                    SchemaStructMemberType::Float
                    | SchemaStructMemberType::Int
                    | SchemaStructMemberType::Bool => default.to_string(),
                    _ => format!("\"{}\"", default),
                };
                args.push(format!("default={}", default));
            } else if matches!(member.ty, SchemaStructMemberType::Color) {
                args.push("default=\"#FFFFFFFF\"".to_string());
            }
            if let Some(description) = &member.description {
                args.push(format!("description=\"{}\"", description));
            }
            if let Some(arguments) = &member.arguments {
                args.push(format!("arguments=\"{}\"", arguments));
            }
            if let Some(min) = &member.minvalue {
                args.push(format!("min={}", min));
            }
            if let Some(max) = &member.maxvalue {
                args.push(format!("max={}", max));
            }
            // if let Some(typeid) = &member.typeid {
            //     args.push(format!("typeid=\"{}\"", typeid));
            // }

            let ty = match member.ty {
                SchemaStructMemberType::Struct => {
                    args.insert(
                        0,
                        typeid_fmt(member.typeid.expect("Struct member should have a typeid")),
                    );
                    "object"
                }
                SchemaStructMemberType::StructList => {
                    generics.push(format!(
                        "object \"Item\" {}",
                        typeid_fmt(
                            member
                                .typeid
                                .expect("Struct list member should have a typeid")
                        )
                    ));
                    "list"
                }
                SchemaStructMemberType::Object => {
                    let id = typeid_fmt(
                        member
                            .typeid
                            .expect("Object ref member should have a typeid"),
                    );
                    if member.options.is_some_and(|opt| opt.contains("notnull")) {
                        args.insert(0, "\"sys:ids/numeric_ref\"".to_string());
                        args.push(format!("typeid={}", id));
                        generics.push(format!("const \"Id\" {}", id));
                        "object"
                    } else {
                        args.insert(0, "\"sys:optional\"".to_string());
                        generics.push(format!(
                            "object \"Item\" \"sys:ids/numeric_ref\" {{\n\tconst \"Id\" {}\n}}",
                            id
                        ));
                        "object"
                    }
                }
                SchemaStructMemberType::ObjectList => {
                    generics.push(format!(
                        "object \"Item\" \"sys:ids/numeric_ref\" {{\n\tconst \"Id\" {}\n}}",
                        typeid_fmt(
                            member
                                .typeid
                                .expect("Object ref list member should have a typeid")
                        )
                    ));
                    "list"
                }
                SchemaStructMemberType::Enum => {
                    args.insert(
                        0,
                        typeid_fmt(member.typeid.expect("Enum member should have a typeid")),
                    );
                    "object"
                }
                SchemaStructMemberType::EnumFlags => {
                    generics.push(format!(
                        "object \"Item\" {}",
                        typeid_fmt(
                            member
                                .typeid
                                .expect("Enum flags member should have a typeid")
                        )
                    ));
                    args.push("editor=\"enum_flags\"".to_string());
                    "list"
                }
                SchemaStructMemberType::Expression => {
                    args.push("editor=\"eh:expression\"".to_string());
                    "string"
                }
                SchemaStructMemberType::Vector => {
                    args.push("\"sys:vec2\"".to_string());
                    "object"
                }
                SchemaStructMemberType::Float => "number",
                SchemaStructMemberType::Int => {
                    args.push("type=\"int\"".to_string());
                    "number"
                }
                SchemaStructMemberType::Color => {
                    args.push("\"color:argb\"".to_string());
                    "object"
                }
                SchemaStructMemberType::Bool => "boolean",
                SchemaStructMemberType::String => "string",
                SchemaStructMemberType::Image => {
                    args.push("editor=\"eh:image\"".to_string());
                    "string"
                }
                SchemaStructMemberType::AudioClip => {
                    args.push("editor=\"eh:audioclip\"".to_string());
                    "string"
                }
                SchemaStructMemberType::Prefab => {
                    args.push("editor=\"eh:prefab\"".to_string());
                    "string"
                }
                SchemaStructMemberType::Layout => {
                    args.push("editor=\"eh:layout\"".to_string());
                    "string"
                }
            };

            code += &format!("\n\t{} \"{}\"", ty, member.name);
            if !args.is_empty() {
                code += " ";
                code += &args.join(" ");
            }
            if !generics.is_empty() {
                code += " {";
                for generic in generics {
                    code += &format!(
                        "\n{}",
                        generic.lines().map(|g| format!("\t\t{}", g)).join("\n")
                    );
                }
                code += "\n\t}";
            }
        }

        code += "\n}";

        self.output(id, code);
    }

    fn emit_enum(&mut self, data: EnumData) {
        let _guard = error_span!("Enum", name = &data.name).entered();

        let code = if let Some((fields, field)) = data.linked_struct {
            let mut code = format!("enum title=\"{}\" tag=\"{}\" {{", &data.name, field);
            for (i, (item, struct_id)) in data.item.into_iter().zip(fields).enumerate() {
                code += &format!(
                    "\n\tobject \"{}\" \"{}\" tag={}",
                    item.name,
                    struct_id,
                    item.value
                        .map_or_else(|| i.to_string(), |s| s.replace('\'', "\""))
                );
            }
            code += "\n}";
            code
        } else {
            let mut code = format!("enum title=\"{}\" {{", &data.name);

            for (i, item) in data.item.into_iter().enumerate() {
                code += &format!(
                    "\n\tconst \"{}\" {}",
                    item.name,
                    item.value
                        .map_or_else(|| i.to_string(), |s| s.replace('\'', "\""))
                );
            }

            code += "\n}";
            code
        };

        self.output(data.id, code);
    }

    fn output(&mut self, id: String, content: String) {
        if let Some(e) = self.files.insert(id, content) {
            panic!("Duplicate file definition for ID `{}`", e)
        }
    }
}

fn id(path: &Utf8Path) -> String {
    "eh:".to_string()
        + &path
            .components()
            .filter(|c| !c.as_str().is_empty())
            .map(|c| c.as_str().to_case(Case::Snake))
            .join("/")
            .replace(".xml", "")
}

fn typeid(typeids: &BTreeMap<String, String>, typeid: String) -> String {
    let _guard = error_span!("Typeid", typeid = &typeid).entered();
    typeids.get(&typeid).expect("Typeid should exist").clone()
}
