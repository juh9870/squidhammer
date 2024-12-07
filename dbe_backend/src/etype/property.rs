use crate::etype::econst::ETypeConst;
use ahash::AHashMap;
use atomic_refcell::AtomicRefCell;
use itertools::Itertools;
use miette::Context;
pub use paste;
use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::LazyLock;
use tracing::error;

pub mod default_properties;
pub mod wrappers;

static ALL_PROPERTIES: LazyLock<AtomicRefCell<AHashMap<(String, PropertyKind), PropertyInfo>>> =
    LazyLock::new(|| AtomicRefCell::new(Default::default()));

pub type PropValidator = fn(ETypeConst) -> miette::Result<()>;

/// Register a field property. Unregistered properties will panic on access.
pub fn register_field_property<T: TryFrom<ETypeConst>>(prop: &FieldProperty<T>) {
    let prop = &prop.0;
    let key = (prop.info.id.to_string(), PropertyKind::Field);
    let id = prop.info.id;
    if ALL_PROPERTIES
        .borrow_mut()
        .insert(key, prop.info.clone())
        .is_some()
    {
        panic!("Field property {} already registered", id);
    };
}

/// Register an object property. Unregistered properties will panic on access.
pub fn register_object_property<T: TryFrom<ETypeConst>>(prop: &ObjectProperty<T>) {
    let prop = &prop.0;
    let key = (prop.info.id.to_string(), PropertyKind::Object);
    let id = prop.info.id;
    if ALL_PROPERTIES
        .borrow_mut()
        .insert(key, prop.info.clone())
        .is_some()
    {
        panic!("Object property {} already registered", id);
    };
}

/// Macro to define extra properties allowed in schema files.
///
/// Properties are defined as static variables with the type `LazyLock<Property<T>>`.
///
/// All properties must be registered with `register_field_property` or `register_object_property`.
///
/// # Example
/// ```rust
/// # use dbe_backend::extra_properties;
/// # use ustr::Ustr;
///
/// extra_properties! {
///     pub prop<object> graph_autoconvert: bool;
///     pub prop<object> graph_autoconvert_variant: Ustr;
///     pub prop<field> graph_inline: bool;
/// }
/// extra_properties! {
///    /// A
///    /// B
///    pub prop<object> graph_autoconvert2: bool
/// }
/// ```
#[macro_export]
macro_rules! extra_properties {
    (@single $(#[doc = $doc:literal])* $vis:vis prop<$kind:tt> $id:ident: $ty:ty) => {
        $crate::etype::property::paste::paste! {
            $(#[doc = $doc])*
            $vis static [< PROP_ $kind:upper _ $id:snake:upper >]: std::sync::LazyLock<$crate::extra_properties!(@ty $kind $ty)> =
                std::sync::LazyLock::new(|| <$crate::extra_properties!(@ty $kind $ty)>::new($crate::etype::property::PropertyInfo {
                id: stringify!($id),
                desc: concat!($($doc, "\n"),*),
                ty: stringify!($ty),
                validator: $crate::extra_properties!(@validator $ty)
            }));
        }
    };
    (@ty object $ty:ty) => {
        $crate::etype::property::ObjectProperty::<$ty>
    };
    (@ty field $ty:ty) => {
        $crate::etype::property::FieldProperty::<$ty>
    };
    (@validator ETypeConst) => {
        |_v| Ok(())
    };
    (@validator $ty:ty) => {
        |v| {
            <$ty as TryFrom<$crate::etype::econst::ETypeConst>>::try_from(v).map(|_| ()).map_err(Into::into)
        }
    };
    ($($(#[doc = $doc:expr])* $vis:vis prop<$kind:tt> $id:ident: $ty:ty);+ $(;)?) => {
        $(
            $crate::extra_properties!(@single $(#[doc = $doc])* $vis prop<$kind> $id: $ty);
        )*

        $crate::etype::property::paste::paste! {
            pub fn register_extra_properties() {
                static ONCE: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
                    $(
                        $crate::etype::property::[<register_ $kind _property>](&[< PROP_ $kind:upper _ $id:snake:upper >]);
                    )*
                });
                let _: () = *ONCE;
            }
        }
    };
}

pub fn field_props(
    props: AHashMap<String, ETypeConst>,
) -> miette::Result<AHashMap<FieldPropertyId, ETypeConst>> {
    let all_props = ALL_PROPERTIES.borrow();
    props
        .into_iter()
        .map(|(k, v)| {
            check_prop(&all_props, &k, PropertyKind::Field, v)?;
            Ok((FieldPropertyId(Cow::Owned(k)), v))
        })
        .try_collect()
}

pub fn object_props(
    props: AHashMap<String, ETypeConst>,
) -> miette::Result<AHashMap<ObjectPropertyId, ETypeConst>> {
    let all_props = ALL_PROPERTIES.borrow();
    props
        .into_iter()
        .map(|(k, v)| {
            check_prop(&all_props, &k, PropertyKind::Object, v)?;
            Ok((ObjectPropertyId(Cow::Owned(k)), v))
        })
        .try_collect()
}

fn check_prop(
    all_props: &AHashMap<(String, PropertyKind), PropertyInfo>,
    id: &str,
    kind: PropertyKind,
    value: ETypeConst,
) -> miette::Result<()> {
    if let Some(prop) = all_props.get(&(id.to_string(), kind)) {
        (prop.validator)(value).with_context(|| format!("failed to parse property {}", id))?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub id: &'static str,
    pub desc: &'static str,
    pub ty: &'static str,
    pub validator: PropValidator,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PropertyKind {
    Field,
    Object,
}

impl PropertyKind {
    pub fn name(&self) -> &'static str {
        match self {
            PropertyKind::Field => "Field",
            PropertyKind::Object => "Object",
        }
    }
}

#[derive(Debug)]
pub struct FieldProperty<T: TryFrom<ETypeConst>>(Property<T>);

impl<T: TryFrom<ETypeConst>> FieldProperty<T> {
    pub fn new(info: PropertyInfo) -> Self {
        Self(Property {
            info,
            _t: PhantomData,
        })
    }

    pub fn info(&self) -> &PropertyInfo {
        &self.0.info
    }
}

#[derive(Debug)]
pub struct ObjectProperty<T: TryFrom<ETypeConst>>(Property<T>);

impl<T: TryFrom<ETypeConst>> ObjectProperty<T> {
    pub fn new(info: PropertyInfo) -> Self {
        Self(Property {
            info,
            _t: PhantomData,
        })
    }

    pub fn info(&self) -> &PropertyInfo {
        &self.0.info
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FieldPropertyId(Cow<'static, str>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ObjectPropertyId(Cow<'static, str>);

impl<T: TryFrom<ETypeConst, Error: Debug>> FieldProperty<T> {
    pub fn get(&self, props: &AHashMap<FieldPropertyId, ETypeConst>, default: T) -> T {
        self.0.assert_registered(PropertyKind::Field);
        self.0.get(
            props.get(&FieldPropertyId(Cow::Borrowed(self.0.info.id))),
            default,
        )
    }

    pub fn try_get(&self, props: &AHashMap<FieldPropertyId, ETypeConst>) -> Option<T> {
        self.0.assert_registered(PropertyKind::Field);
        self.0
            .try_get(props.get(&FieldPropertyId(Cow::Borrowed(self.0.info.id))))
    }
}

impl<T: TryFrom<ETypeConst, Error: Debug>> ObjectProperty<T> {
    pub fn get(&self, props: &AHashMap<ObjectPropertyId, ETypeConst>, default: T) -> T {
        self.0.assert_registered(PropertyKind::Object);
        self.0.get(
            props.get(&ObjectPropertyId(Cow::Borrowed(self.0.info.id))),
            default,
        )
    }

    pub fn try_get(&self, props: &AHashMap<ObjectPropertyId, ETypeConst>) -> Option<T> {
        self.0.assert_registered(PropertyKind::Object);
        self.0
            .try_get(props.get(&ObjectPropertyId(Cow::Borrowed(self.0.info.id))))
    }
}

#[derive(Debug)]
struct Property<T: TryFrom<ETypeConst>> {
    info: PropertyInfo,
    _t: PhantomData<T>,
}

impl<T: TryFrom<ETypeConst, Error: Debug>> Property<T> {
    fn assert_registered(&self, kind: PropertyKind) {
        #[cfg(debug_assertions)]
        {
            let all_properties = ALL_PROPERTIES.borrow();
            let key = (self.info.id.to_string(), kind);
            if !all_properties.contains_key(&key) {
                panic!("{} property {} not registered", kind.name(), self.info.id);
            }
        }
    }

    fn get(&self, prop: Option<&ETypeConst>, default: T) -> T {
        let Some(prop) = prop else {
            return default;
        };

        match T::try_from(*prop) {
            Ok(value) => value,
            Err(err) => {
                error!(
                    id = self.info.id,
                    ?err,
                    "!!INTERNAL ERROR!! property type should have been checked before"
                );
                default
            }
        }
    }

    fn try_get(&self, prop: Option<&ETypeConst>) -> Option<T> {
        let prop = match prop {
            Some(prop) => prop,
            None => return None,
        };

        match T::try_from(*prop) {
            Ok(value) => Some(value),
            Err(err) => {
                error!(
                    id = self.info.id,
                    ?err,
                    "!!INTERNAL ERROR!! property type should have been checked before"
                );
                None
            }
        }
    }
}
