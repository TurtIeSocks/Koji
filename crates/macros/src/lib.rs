use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Data, DeriveInput, Fields, Ident, ItemFn, LitStr, MetaNameValue, Token, Type, braced,
    parse_macro_input,
};

#[proc_macro_attribute]
pub fn time(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute argument (the message string)
    let message = parse_macro_input!(attr as Option<LitStr>);
    // Parse the function
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    let message_str = message
        .map(|f| f.value())
        .unwrap_or(fn_sig.ident.to_string());

    // Generate the new function with timing code
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #[cfg(not(target_arch = "wasm32"))]
            let __timer_start = std::time::Instant::now();
            #[cfg(not(target_arch = "wasm32"))]
            log::info!("starting {}", #message_str);

            // Original function body wrapped to capture return value
            let __timer_result = (|| #fn_block)();

            #[cfg(not(target_arch = "wasm32"))]
            log::info!("finished {} in {:.2}s", #message_str, __timer_start.elapsed().as_secs_f32());

            __timer_result
        }
    };

    TokenStream::from(expanded)
}

/// Derives string conversions for an enum whose variants carry `#[str(...)]`.
///
/// Most variants are unit variants tagged `#[str("literal")]`, optionally with
/// `alias(...)` extra spellings: `#[str("canonical", alias("a", "b"))]`.
///
/// Generates `as_str`, `from_str_opt`, `Serialize` (as the canonical string),
/// `Deserialize` (from string), and `Display`. `from_str_opt` lowercases its
/// input before matching, so all `#[str]` literals must be lowercase; aliases
/// are matched the same way. Serialize/`as_str`/Display use only the canonical
/// literal.
///
/// Exactly ONE newtype variant `Variant(String)` may be tagged `#[str(default)]`.
/// When present it is a catch-all: unknown input deserializes / `from_str_opt`s
/// into `Variant(orig)` preserving the ORIGINAL-case input (so `from_str_opt`
/// always returns `Some` and `Deserialize` never errors). With a default,
/// `as_str` returns the inner `&str` and is therefore `fn as_str(&self) -> &str`
/// (non-const); without one it stays `const fn as_str(&self) -> &'static str`.
///
/// Does NOT derive `PartialEq`/`Eq`/`Hash`/`Debug`/`Clone` — declare those
/// normally via `#[derive(...)]` on the enum.
#[proc_macro_derive(StrEnum, attributes(str))]
pub fn derive_str_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let data = match &input.data {
        Data::Enum(d) => d,
        _ => {
            return syn::Error::new_spanned(&input, "StrEnum only supports enums")
                .to_compile_error()
                .into();
        }
    };

    // Canonical (unit) variants.
    let mut as_str_arms = Vec::new();
    let mut from_arms = Vec::new();
    // The single optional `#[str(default)]` catch-all newtype variant.
    let mut default_ident: Option<syn::Ident> = None;

    for variant in &data.variants {
        let vident = &variant.ident;

        // Parse this variant's `#[str(...)]` attribute into either a default
        // marker or a canonical literal plus optional aliases.
        let mut is_default = false;
        let mut lit: Option<LitStr> = None;
        let mut aliases: Vec<LitStr> = Vec::new();
        let mut had_str_attr = false;

        for attr in &variant.attrs {
            if !attr.path().is_ident("str") {
                continue;
            }
            had_str_attr = true;
            // Grammar: `default` | `"lit"` | `"lit", alias("a", "b", ...)`.
            let parsed = attr.parse_args_with(|input: syn::parse::ParseStream| {
                if input.peek(syn::Ident) {
                    let kw: syn::Ident = input.parse()?;
                    if kw == "default" {
                        return Ok((true, None, Vec::new()));
                    }
                    return Err(syn::Error::new_spanned(
                        kw,
                        "expected `default` or a string literal in #[str(...)]",
                    ));
                }
                let canonical: LitStr = input.parse()?;
                let mut als: Vec<LitStr> = Vec::new();
                if input.peek(syn::Token![,]) {
                    let _comma: syn::Token![,] = input.parse()?;
                    let kw: syn::Ident = input.parse()?;
                    if kw != "alias" {
                        return Err(syn::Error::new_spanned(
                            kw,
                            "expected `alias(...)` after the canonical literal",
                        ));
                    }
                    let content;
                    syn::parenthesized!(content in input);
                    let punctuated = content
                        .parse_terminated(<LitStr as syn::parse::Parse>::parse, syn::Token![,])?;
                    als.extend(punctuated);
                }
                Ok((false, Some(canonical), als))
            });
            match parsed {
                Ok((d, l, a)) => {
                    is_default = d;
                    lit = l;
                    aliases = a;
                }
                Err(e) => return e.to_compile_error().into(),
            }
        }

        if !had_str_attr {
            return syn::Error::new_spanned(variant, "missing #[str(\"...\")] attribute")
                .to_compile_error()
                .into();
        }

        if is_default {
            // Must be a single-field tuple variant `Variant(String)`.
            match &variant.fields {
                Fields::Unnamed(f) if f.unnamed.len() == 1 => {}
                _ => {
                    return syn::Error::new_spanned(
                        variant,
                        "#[str(default)] requires a single-field newtype variant like `Custom(String)`",
                    )
                    .to_compile_error()
                    .into();
                }
            }
            if default_ident.is_some() {
                return syn::Error::new_spanned(
                    variant,
                    "only one #[str(default)] variant is allowed",
                )
                .to_compile_error()
                .into();
            }
            default_ident = Some(vident.clone());
            continue;
        }

        // Canonical unit variant.
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(
                variant,
                "StrEnum variants must be unit variants (only the #[str(default)] variant may hold a String)",
            )
            .to_compile_error()
            .into();
        }
        let lit = match lit {
            Some(s) => s,
            None => {
                return syn::Error::new_spanned(variant, "missing #[str(\"...\")] literal")
                    .to_compile_error()
                    .into();
            }
        };
        as_str_arms.push(quote! { #name::#vident => #lit });
        // Canonical literal plus any aliases all map to this variant.
        from_arms.push(quote! { #lit => ::core::option::Option::Some(#name::#vident) });
        for a in &aliases {
            from_arms.push(quote! { #a => ::core::option::Option::Some(#name::#vident) });
        }
    }

    // Build the two halves that differ by whether a default catch-all exists.
    let (as_str_impl, from_str_body) = if let Some(default) = &default_ident {
        // Non-const as_str: the catch-all arm borrows the inner String.
        let as_str = quote! {
            pub fn as_str(&self) -> &str {
                match self {
                    #(#as_str_arms,)*
                    #name::#default(__s) => __s.as_str(),
                }
            }
        };
        // Lowercase for matching; catch-all keeps the ORIGINAL-case input.
        let from_body = quote! {
            match s.to_lowercase().as_str() {
                #(#from_arms,)*
                _ => ::core::option::Option::Some(#name::#default(s.to_owned())),
            }
        };
        (as_str, from_body)
    } else {
        let as_str = quote! {
            pub const fn as_str(&self) -> &'static str {
                match self { #(#as_str_arms),* }
            }
        };
        let from_body = quote! {
            match s.to_lowercase().as_str() {
                #(#from_arms,)*
                _ => ::core::option::Option::None,
            }
        };
        (as_str, from_body)
    };

    // Deserialize: with a default it can never fail; without, unknown errors.
    let deserialize_tail = if default_ident.is_some() {
        quote! {
            // `from_str_opt` is total when a default exists.
            ::core::result::Result::Ok(Self::from_str_opt(&s).unwrap())
        }
    } else {
        quote! {
            Self::from_str_opt(&s).ok_or_else(|| {
                <D::Error as ::serde::de::Error>::custom(
                    ::std::format!("unknown {}: {}", ::core::stringify!(#name), s)
                )
            })
        }
    };

    quote! {
        impl #name {
            #as_str_impl
            pub fn from_str_opt(s: &str) -> ::core::option::Option<Self> {
                #from_str_body
            }
        }
        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, ser: S) -> ::core::result::Result<S::Ok, S::Error> {
                ser.serialize_str(self.as_str())
            }
        }
        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(de: D) -> ::core::result::Result<Self, D::Error> {
                let s = <::std::string::String as ::serde::Deserialize>::deserialize(de)?;
                #deserialize_tail
            }
        }
        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    }
    .into()
}

/// Generates the verbatim-identical `impl Query { get_one, get_one_json,
/// delete, search }` CRUD block shared by the admin-CRUD entities (property,
/// project, tile_server), which differ in nothing but their entity module.
/// Place on `pub struct Query;`:
///
/// ```ignore
/// #[macros::crud_query]
/// pub struct Query;
/// ```
///
/// The original `struct Query;` item is re-emitted unchanged, then the impl is
/// appended. `Entity`/`Column`/`Model`/`Json`/`DatabaseConnection`/`DbErr`/
/// `DeleteResult` are emitted unqualified and resolve at the call site (each
/// entity module has `use super::*` + `use sea_orm::entity::prelude::*`, whose
/// prelude re-exports `JsonValue as Json`, `DeleteResult`, `DbErr`,
/// `DatabaseConnection`). `crate::error::ModelError` and `serde_json::json!`
/// are fully qualified so the macro names no caller-local item.
///
/// Note: the shared not-found arm reproduces the pre-existing
/// `ModelError::Geofence("Does not exist")` misnomer verbatim — these CRUD
/// entities are not geofences, but the variant predates this extraction and is
/// intentionally left unchanged.
#[proc_macro_attribute]
pub fn crud_query(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: proc_macro2::TokenStream = item.into();
    let expanded = quote! {
        #item

        impl Query {
            pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, crate::error::ModelError> {
                let record = match id.parse::<u32>() {
                    Ok(id) => Entity::find_by_id(id).one(db).await?,
                    Err(_) => Entity::find().filter(Column::Name.eq(id)).one(db).await?,
                };
                if let Some(record) = record {
                    Ok(record)
                } else {
                    Err(crate::error::ModelError::Geofence("Does not exist".to_string()))
                }
            }
            pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, crate::error::ModelError> {
                Ok(serde_json::json!(Self::get_one(db, id).await?))
            }
            pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
                Entity::delete_by_id(id).exec(db).await
            }
            pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
                Entity::find()
                    .filter(Column::Name.like(format!("%{}%", search).as_str()))
                    .into_json()
                    .all(db)
                    .await
            }
        }
    };
    expanded.into()
}

/// Generates the verbatim-identical `impl Query { all, bound, area, stats }`
/// block shared by fort-shaped golbat entities (gym, pokestop), which differ
/// only by raw-SQL table name and id prefix. Place on `pub struct Query;`:
///
/// ```ignore
/// #[macros::fort_query(table = "gym", prefix = "g")]
/// pub struct Query;
/// ```
///
/// The original `struct Query;` item is re-emitted unchanged, then the impl is
/// appended. `Entity`/`Column` are emitted unqualified and resolve at the call
/// site (via the entity module's `use sea_orm::entity::prelude::*`); the query
/// -builder methods need `QueryFilter`/`QuerySelect` in scope there too. All
/// `sea_orm`/`crate` items the impl references are fully qualified, so this
/// macro never names a koji-golbat type directly (no crate dep, no cycle).
#[proc_macro_attribute]
pub fn fort_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse `table = "gym", prefix = "g"` as two `Ident = LitStr` pairs.
    let args = parse_macro_input!(
        attr with Punctuated::<MetaNameValue, Token![,]>::parse_terminated
    );

    let mut table: Option<LitStr> = None;
    let mut prefix: Option<LitStr> = None;
    for nv in args {
        let key = match nv.path.get_ident() {
            Some(id) => id.to_string(),
            None => {
                return syn::Error::new_spanned(&nv.path, "expected `table` or `prefix`")
                    .to_compile_error()
                    .into();
            }
        };
        // Each value must be a string literal (`"gym"` / `"g"`).
        let lit = match &nv.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => s.clone(),
            other => {
                return syn::Error::new_spanned(other, "expected a string literal")
                    .to_compile_error()
                    .into();
            }
        };
        match key.as_str() {
            "table" => table = Some(lit),
            "prefix" => prefix = Some(lit),
            _ => {
                return syn::Error::new_spanned(&nv.path, "expected `table` or `prefix`")
                    .to_compile_error()
                    .into();
            }
        }
    }

    let table = match table {
        Some(t) => t,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "fort_query requires `table = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };
    let prefix = match prefix {
        Some(p) => p,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "fort_query requires `prefix = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };

    // Re-emit the original item (the `struct Query;`) unchanged.
    let item: proc_macro2::TokenStream = item.into();

    let expanded = quote! {
        #item

        impl Query {
            pub async fn all(
                conn: &sea_orm::DatabaseConnection,
                last_seen: u32,
            ) -> Result<Vec<crate::rows::GenericData>, sea_orm::DbErr> {
                let items = Entity::find()
                    .select_only()
                    .column(Column::Lat)
                    .column(Column::Lon)
                    .filter(Column::Updated.gt(last_seen))
                    .filter(Column::Deleted.eq(false))
                    .filter(Column::Enabled.eq(true))
                    .limit(2_000_000)
                    .into_model::<crate::rows::LatLonRow>()
                    .all(conn)
                    .await?;
                Ok(crate::normalize::fort(items, #prefix))
            }

            pub async fn bound(
                conn: &sea_orm::DatabaseConnection,
                payload: &koji_core::BoundsArg,
            ) -> Result<Vec<crate::rows::GenericData>, sea_orm::DbErr> {
                let items = Entity::find()
                    .select_only()
                    .column(Column::Lat)
                    .column(Column::Lon)
                    .filter(Column::Lat.between(payload.bbox.min_lat, payload.bbox.max_lat))
                    .filter(Column::Lon.between(payload.bbox.min_lon, payload.bbox.max_lon))
                    .filter(Column::Updated.gt(payload.last_seen.unwrap_or_default()))
                    .filter(Column::Deleted.eq(false))
                    .filter(Column::Enabled.eq(true))
                    .limit(2_000_000)
                    .into_model::<crate::rows::LatLonRow>()
                    .all(conn)
                    .await?;
                Ok(crate::normalize::fort(items, #prefix))
            }

            pub async fn area(
                conn: &sea_orm::DatabaseConnection,
                area: &geojson::FeatureCollection,
                last_seen: u32,
            ) -> Result<Vec<crate::rows::GenericData>, sea_orm::DbErr> {
                let items = Entity::find()
                    .from_raw_sql(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DbBackend::MySql,
                        format!(
                            "SELECT lat, lon FROM {} WHERE enabled = 1 AND deleted = 0 AND updated > {} AND ({})",
                            #table,
                            last_seen,
                            crate::sql_raw_bbox(area)
                        )
                        .as_str(),
                        vec![],
                    ))
                    .into_model::<crate::rows::LatLonRow>()
                    .all(conn)
                    .await?;
                Ok(crate::normalize::fort_filtered(items, area, #prefix))
            }

            pub async fn stats(
                conn: &sea_orm::DatabaseConnection,
                area: &geojson::FeatureCollection,
                last_seen: u32,
            ) -> Result<crate::rows::Total, sea_orm::DbErr> {
                let items = Entity::find()
                    .from_raw_sql(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DbBackend::MySql,
                        format!(
                            "SELECT lat, lon FROM {} WHERE enabled = 1 AND deleted = 0 AND updated > {} AND ({})",
                            #table,
                            last_seen,
                            crate::sql_raw_bbox(area)
                        )
                        .as_str(),
                        vec![],
                    ))
                    .into_model::<crate::rows::LatLonRow>()
                    .all(conn)
                    .await?;
                let total = crate::count_in_area(&items, area);
                Ok(crate::rows::Total { total })
            }
        }
    };

    expanded.into()
}

// ===========================================================================
// koji_resource! — typed CRUD DTOs + the five REST handlers + scope() for a
// plain (non-geometry) v2 resource.
// ===========================================================================

/// One `name: Type` entry in the `create: { … }` field list.
struct ResourceField {
    name: Ident,
    ty: Type,
}

/// The parsed `koji_resource! { module:, seg:, topic:, outbox:, create: { … } }`
/// invocation.
struct ResourceDef {
    /// koji-db `db::<module>::Query` module + emitted submodule name (the
    /// singular canonical resource name, e.g. `project`).
    module: Ident,
    /// URL path segment, e.g. `"projects"`.
    seg: LitStr,
    /// Realtime event topic name, e.g. `"project"`.
    topic: LitStr,
    /// Optional `outbox: true` flag (default `false`). When set, `update`/
    /// `remove` additionally publish `"{topic}.updated"` / `"{topic}.deleted"`
    /// to the `koji_events` outbox. Only the `project` invocation opts in.
    outbox: bool,
    /// The Create DTO fields, in declaration order.
    fields: Vec<ResourceField>,
}

impl Parse for ResourceDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut module: Option<Ident> = None;
        let mut seg: Option<LitStr> = None;
        let mut topic: Option<LitStr> = None;
        let mut outbox: Option<bool> = None;
        let mut fields: Option<Vec<ResourceField>> = None;

        // Grammar: a comma-separated list of `key: value`, where `value` is an
        // ident (`module`), a string literal (`seg`, `topic`), a bool literal
        // (`outbox`), or a `{ … }` field block (`create`). A trailing comma is
        // allowed.
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "module" => module = Some(input.parse()?),
                "seg" => seg = Some(input.parse()?),
                "topic" => topic = Some(input.parse()?),
                "outbox" => {
                    let lit: syn::LitBool = input.parse()?;
                    outbox = Some(lit.value);
                }
                "create" => {
                    let content;
                    braced!(content in input);
                    let parsed = content.parse_terminated(
                        |f: ParseStream| {
                            let name: Ident = f.parse()?;
                            f.parse::<Token![:]>()?;
                            let ty: Type = f.parse()?;
                            Ok(ResourceField { name, ty })
                        },
                        Token![,],
                    )?;
                    fields = Some(parsed.into_iter().collect());
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "unexpected key `{other}` (expected `module`, `seg`, `topic`, `outbox`, or `create`)"
                        ),
                    ));
                }
            }
            // Consume the separating comma between top-level entries, if any.
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let module = module.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "koji_resource! requires `module:`",
            )
        })?;
        let seg = seg.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "koji_resource! requires `seg:`",
            )
        })?;
        let topic = topic.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "koji_resource! requires `topic:`",
            )
        })?;
        let fields = fields.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "koji_resource! requires `create: { … }`",
            )
        })?;

        Ok(ResourceDef {
            module,
            seg,
            topic,
            outbox: outbox.unwrap_or(false),
            fields,
        })
    }
}

/// Whether a type is already `Option<…>` (so the Patch DTO leaves it alone
/// rather than double-wrapping). Detects the last path segment being `Option`,
/// matching `Option<T>` / `std::option::Option<T>` / `core::option::Option<T>`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty
        && let Some(last) = tp.path.segments.last()
    {
        return last.ident == "Option";
    }
    false
}

/// The leaf-type idents `utoipa`'s `ToSchema` derive already understands as
/// JSON primitives (so no `value_type` override is needed). Anything else in a
/// `koji_resource!` field is — by the established plain-resource convention — a
/// string-serializing newtype/`StrEnum` (e.g. `koji_db::Category`) whose owning
/// crate carries no `utoipa` dep; those get a `#[schema(value_type = String)]`
/// hint so the generated DTO derives `ToSchema` without forcing utoipa into the
/// data crates (P8 openapi, documentation-only).
fn is_schema_primitive(ident: &str) -> bool {
    matches!(
        ident,
        "String"
            | "str"
            | "bool"
            | "char"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}

/// The optional `#[schema(value_type = String)]` attribute for a field whose
/// (possibly `Option<…>`-wrapped) leaf type is not a JSON primitive utoipa knows
/// — see [`is_schema_primitive`]. Empty for primitive fields.
fn schema_value_type_attr(ty: &Type) -> proc_macro2::TokenStream {
    // Unwrap one `Option<…>` layer to inspect the inner leaf type.
    let leaf = if is_option_type(ty) {
        match ty {
            Type::Path(tp) => tp
                .path
                .segments
                .last()
                .and_then(|seg| match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) => args.args.first(),
                    _ => None,
                })
                .and_then(|arg| match arg {
                    syn::GenericArgument::Type(inner) => Some(inner.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| ty.clone()),
            _ => ty.clone(),
        }
    } else {
        ty.clone()
    };

    let is_primitive = matches!(&leaf, Type::Path(tp) if tp
        .path
        .segments
        .last()
        .is_some_and(|seg| is_schema_primitive(&seg.ident.to_string())));

    if is_primitive {
        quote! {}
    } else {
        quote! { #[schema(value_type = String)] }
    }
}

/// Convert a `snake_case` ident to `PascalCase` (e.g. `tile_server` →
/// `TileServer`), for the `Create…`/`Patch…` DTO type names.
fn pascal_case(ident: &Ident) -> Ident {
    let mut out = String::new();
    let mut upper_next = true;
    for ch in ident.to_string().chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    format_ident!("{}", out)
}

/// Generate the typed Create/Patch DTOs, the five REST handlers, and a
/// `scope()` for a plain (non-geometry) v2 resource, wired to `ServiceError`
/// (404 on misses), `201`+`Location`, `204`, the v2 envelope, and `?page=&
/// per_page=` pagination.
///
/// Invocation (in `koji-service`):
///
/// ```ignore
/// koji_resource! {
///     module: project,          // koji-db `db::project::Query`
///     seg: "projects",          // URL segment
///     create: {                 // Create DTO fields (required unless `Option<…>`)
///         name: String,
///         api_endpoint: Option<String>,
///         golbat: bool,
///     }
/// }
/// ```
///
/// Emits `pub(crate) mod <module> { … }` containing `Create<Module>` (the listed
/// fields verbatim), `Patch<Module>` (each field `Option<…>`, skipped when
/// `None`), the `list`/`create`/`get_one`/`update`/`remove` handlers (all
/// `-> Result<HttpResponse, crate::utils::error::ServiceError>`), and `scope()`.
///
/// Like `crud_query`/`fort_query`, generated code is **call-site-resolved**:
/// koji-service types are named by `crate::…` path and external crates fully
/// qualified (`koji_db::…`, `actix_web::…`, `serde_json::…`), so the macro names
/// no caller-local prelude item.
///
/// **Known shortcut (flagged):** the not-found mapping sniffs the koji-db
/// `ModelError` whose `to_string()` contains `"Does not exist"` (its miss
/// sentinel) and maps it to `ServiceError::NotFound` (404); every other
/// `ModelError` flows through `ServiceError`'s `#[from]` (500). A dedicated
/// `ModelError::NotFound` variant is a candidate follow-up.
#[proc_macro]
pub fn koji_resource(input: TokenStream) -> TokenStream {
    let ResourceDef {
        module,
        seg,
        topic,
        outbox,
        fields,
    } = parse_macro_input!(input as ResourceDef);

    let pascal = pascal_case(&module);
    let create_ty = format_ident!("Create{}", pascal);
    let patch_ty = format_ident!("Patch{}", pascal);
    // The singular canonical resource name (the module ident) labels the 404
    // `field` — always correct, unlike naive depluralizing of `seg`.
    let field_name = module.to_string();

    // Absolute OpenAPI paths for the `#[utoipa::path]` annotations (built from
    // `seg` at expansion time, since utoipa's `path =` wants a string literal).
    let coll_path = LitStr::new(&format!("/api/v2/{}", seg.value()), seg.span());
    let item_path = LitStr::new(&format!("/api/v2/{}/{{id}}", seg.value()), seg.span());
    // The OpenAPI tag groups this resource's operations (the URL segment).
    let tag = seg.clone();

    // `outbox: true` (project only) additionally publishes to the
    // `koji_events` outbox from `update`/`remove`. Absent/false emits nothing
    // here, keeping the non-outbox expansion byte-identical to before this
    // flag existed.
    let outbox_update_publish = if outbox {
        quote! {
            if let ::core::result::Result::Err(e) = koji_events::EventDispatcher::publish(
                &db.koji,
                concat!(#topic, ".updated"),
                &serde_json::json!({
                    "projectId": id,
                    "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
                }),
            )
            .await
            {
                log::warn!(concat!("[", #seg, "] outbox publish failed: {}"), e);
            }
        }
    } else {
        quote! {}
    };
    let outbox_remove_prefetch = if outbox {
        quote! {
            let __name = koji_db::db::#module::Query::get_one(&db.koji, id.to_string())
                .await
                .ok()
                .map(|m| serde_json::json!(m.name))
                .unwrap_or(serde_json::Value::Null);
        }
    } else {
        quote! {}
    };
    let outbox_remove_publish = if outbox {
        quote! {
            if let ::core::result::Result::Err(e) = koji_events::EventDispatcher::publish(
                &db.koji,
                concat!(#topic, ".deleted"),
                &serde_json::json!({ "projectId": id, "name": __name }),
            )
            .await
            {
                log::warn!(concat!("[", #seg, "] outbox publish failed: {}"), e);
            }
        }
    } else {
        quote! {}
    };

    // Create DTO fields, verbatim (+ a `value_type` hint on non-primitive leaves
    // so `ToSchema` derives without forcing utoipa into the data crates).
    let create_fields = fields.iter().map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        let schema_attr = schema_value_type_attr(ty);
        quote! { #schema_attr pub #name: #ty }
    });

    // Patch DTO fields: wrap non-Option in Option<…>; skip when None.
    let patch_fields = fields.iter().map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        let schema_attr = schema_value_type_attr(ty);
        let opt_ty = if is_option_type(ty) {
            quote! { #ty }
        } else {
            quote! { ::core::option::Option<#ty> }
        };
        quote! {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            #schema_attr
            pub #name: #opt_ty
        }
    });

    let expanded = quote! {
        /// Generated typed CRUD (DTOs + five handlers + scope) for this v2
        /// resource. See [`koji_resource!`](macros::koji_resource).
        pub(crate) mod #module {
            use ::serde::{Deserialize, Serialize};
            use ::utoipa::ToSchema;

            /// Typed create body for this resource (deserialized from the
            /// request JSON; serialized to the koji-db upsert value).
            #[derive(Debug, Deserialize, Serialize, ToSchema)]
            pub(crate) struct #create_ty {
                #(#create_fields),*
            }

            /// Typed patch body: every create field optional; omitted fields
            /// stay `None` and are dropped from the serialized upsert value.
            #[derive(Debug, Deserialize, Serialize, ToSchema)]
            pub(crate) struct #patch_ty {
                #(#patch_fields),*
            }

            /// Query parameters for the `list` handler: pagination + sort + filter.
            #[derive(::core::default::Default, ::serde::Deserialize)]
            pub(crate) struct ListQuery {
                pub page: ::core::option::Option<i64>,
                pub per_page: ::core::option::Option<i64>,
                #[serde(alias = "sortBy")]
                pub sort_by: ::core::option::Option<String>,
                pub order: ::core::option::Option<String>,
                pub q: ::core::option::Option<String>,
                pub project: ::core::option::Option<u32>,
                pub parent: ::core::option::Option<u32>,
                pub geotype: ::core::option::Option<String>,
                pub mode: ::core::option::Option<String>,
            }

            /// `GET /api/v2/#seg` — paginated list (`?page=&per_page=&sortBy=&order=&q=`).
            #[utoipa::path(
                get,
                path = #coll_path,
                tag = #tag,
                params(
                    ("page" = ::core::option::Option<i64>, Query, description = "1-based page number"),
                    ("per_page" = ::core::option::Option<i64>, Query, description = "Page size (clamped to [1, 500])"),
                    ("sortBy" = ::core::option::Option<String>, Query, description = "Column to sort by (default: id)"),
                    ("order" = ::core::option::Option<String>, Query, description = "Sort direction: ASC or DESC (default: ASC)"),
                    ("q" = ::core::option::Option<String>, Query, description = "Free-text search filter"),
                ),
                responses(
                    (status = 200, description = "Paginated records (with a `meta` block)", body = Object),
                ),
            )]
            pub(crate) async fn list(
                db: actix_web::web::Data<koji_db::KojiDb>,
                query: actix_web::web::Query<ListQuery>,
            ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
                let page = query.page.unwrap_or(1).max(1);
                let per_page = query.per_page.unwrap_or(50).clamp(1, 500);
                // koji-db `paginate` is 0-based; bridge from the 1-based wire.
                let args = koji_db::query_args::AdminReqParsed {
                    page: (page - 1) as u64,
                    per_page: per_page as u64,
                    sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
                    order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
                    q: query.q.clone().unwrap_or_default(),
                    geotype: query.geotype.clone(),
                    project: query.project,
                    mode: query.mode.clone(),
                    parent: query.parent,
                    geofenceid: ::core::option::Option::None,
                    pointsmin: ::core::option::Option::None,
                    pointsmax: ::core::option::Option::None,
                };
                let (results, total, _has_next, _has_prev) =
                    koji_db::db::#module::Query::paginate(&db.koji, args).await?.into_parts();
                ::core::result::Result::Ok(crate::utils::api_response::ApiResponse::success_paginated(
                    results,
                    crate::utils::api_response::Meta::build(total as i64, page, per_page),
                ))
            }

            /// `POST /api/v2/#seg` — create → `201` + `Location` header.
            #[utoipa::path(
                post,
                path = #coll_path,
                tag = #tag,
                request_body = #create_ty,
                responses(
                    (status = 201, description = "Created; `Location` header points at the new record", body = Object),
                    (status = 500, description = "Internal error", body = crate::utils::api_response::ApiError),
                ),
            )]
            pub(crate) async fn create(
                db: actix_web::web::Data<koji_db::KojiDb>,
                hub: actix_web::web::Data<crate::internal::realtime::RealtimeHub>,
                body: actix_web::web::Json<#create_ty>,
            ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
                let value = serde_json::to_value(&body.into_inner())
                    .map_err(crate::utils::error::ServiceError::internal)?;
                let record = koji_db::db::#module::Query::upsert_json_return(&db.koji, 0, value).await?;
                let id = record.get("id").and_then(serde_json::Value::as_u64).unwrap_or(0);
                for (t, ev) in crate::internal::realtime::topics::created(#topic, id as i64) {
                    hub.publish(&t, ev);
                }
                ::core::result::Result::Ok(
                    actix_web::HttpResponse::build(actix_web::http::StatusCode::CREATED)
                        .insert_header(("Location", format!(concat!("/api/v2/", #seg, "/{}"), id)))
                        .json(crate::utils::api_response::ApiResponse::Ok {
                            data: record,
                            meta: ::core::option::Option::None,
                        }),
                )
            }

            /// `GET /api/v2/#seg/{id}` — fetch one (id or name); `404` on miss.
            #[utoipa::path(
                get,
                path = #item_path,
                tag = #tag,
                params(("id" = String, Path, description = "Record id or name")),
                responses(
                    (status = 200, description = "The record", body = Object),
                    (status = 404, description = "No such record", body = crate::utils::api_response::ApiError),
                ),
            )]
            pub(crate) async fn get_one(
                db: actix_web::web::Data<koji_db::KojiDb>,
                path: actix_web::web::Path<String>,
            ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
                match koji_db::db::#module::Query::get_one_json(&db.koji, path.into_inner()).await {
                    ::core::result::Result::Ok(record) =>
                        ::core::result::Result::Ok(crate::utils::api_response::ApiResponse::success(record)),
                    ::core::result::Result::Err(e) => ::core::result::Result::Err(__not_found_or(e)),
                }
            }

            /// `PATCH /api/v2/#seg/{id}` — partial update; `404` on miss.
            #[utoipa::path(
                patch,
                path = #item_path,
                tag = #tag,
                params(("id" = u32, Path, description = "Record id")),
                request_body = #patch_ty,
                responses(
                    (status = 200, description = "Updated record", body = Object),
                    (status = 404, description = "No such record", body = crate::utils::api_response::ApiError),
                ),
            )]
            pub(crate) async fn update(
                db: actix_web::web::Data<koji_db::KojiDb>,
                hub: actix_web::web::Data<crate::internal::realtime::RealtimeHub>,
                path: actix_web::web::Path<u32>,
                body: actix_web::web::Json<#patch_ty>,
            ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
                let id = path.into_inner();
                // `upsert_json_return` is upsert (a missing id would INSERT), so
                // pre-check existence to honor PATCH's 404-on-missing contract.
                if let ::core::result::Result::Err(e) =
                    koji_db::db::#module::Query::get_one(&db.koji, id.to_string()).await
                {
                    return ::core::result::Result::Err(__not_found_or(e));
                }
                let value = serde_json::to_value(&body.into_inner())
                    .map_err(crate::utils::error::ServiceError::internal)?;
                let record = koji_db::db::#module::Query::upsert_json_return(&db.koji, id, value).await?;
                for (t, ev) in crate::internal::realtime::topics::updated(#topic, id as i64, record.clone()) {
                    hub.publish(&t, ev);
                }
                #outbox_update_publish
                ::core::result::Result::Ok(crate::utils::api_response::ApiResponse::success(record))
            }

            /// `DELETE /api/v2/#seg/{id}` — `204 No Content`; `404` on miss.
            #[utoipa::path(
                delete,
                path = #item_path,
                tag = #tag,
                params(("id" = u32, Path, description = "Record id")),
                responses(
                    (status = 204, description = "Deleted"),
                    (status = 404, description = "No such record", body = crate::utils::api_response::ApiError),
                ),
            )]
            pub(crate) async fn remove(
                db: actix_web::web::Data<koji_db::KojiDb>,
                hub: actix_web::web::Data<crate::internal::realtime::RealtimeHub>,
                path: actix_web::web::Path<u32>,
            ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
                let id = path.into_inner();
                #outbox_remove_prefetch
                let result = koji_db::db::#module::Query::delete(&db.koji, id).await?;
                if result.rows_affected == 0 {
                    return ::core::result::Result::Err(crate::utils::error::ServiceError::NotFound {
                        field: #field_name,
                        message: "does not exist".to_string(),
                    });
                }
                for (t, ev) in crate::internal::realtime::topics::deleted(#topic, id as i64) {
                    hub.publish(&t, ev);
                }
                #outbox_remove_publish
                ::core::result::Result::Ok(
                    actix_web::HttpResponse::build(actix_web::http::StatusCode::NO_CONTENT).finish(),
                )
            }

            /// Map a koji-db `ModelError` to a `ServiceError`: its
            /// `"Does not exist"` miss sentinel → `NotFound` (404); everything
            /// else flows through `#[from] ModelError` (500).
            fn __not_found_or(e: koji_db::ModelError) -> crate::utils::error::ServiceError {
                if e.to_string().contains("Does not exist") {
                    crate::utils::error::ServiceError::NotFound {
                        field: #field_name,
                        message: "does not exist".to_string(),
                    }
                } else {
                    crate::utils::error::ServiceError::from(e)
                }
            }

            /// `web::Scope` wiring the five handlers under `/#seg`, mounted into
            /// `/api/v2` by [`crate::start`].
            pub(crate) fn scope() -> actix_web::Scope {
                actix_web::web::scope(concat!("/", #seg))
                    .service(
                        actix_web::web::resource("")
                            .route(actix_web::web::get().to(list))
                            .route(actix_web::web::post().to(create)),
                    )
                    .service(
                        actix_web::web::resource("/{id}")
                            .route(actix_web::web::get().to(get_one))
                            .route(actix_web::web::patch().to(update))
                            .route(actix_web::web::delete().to(remove)),
                    )
            }
        }
    };

    expanded.into()
}
