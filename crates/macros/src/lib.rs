use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Data, DeriveInput, Fields, ItemFn, LitStr, MetaNameValue, Token, parse_macro_input,
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
                    let punctuated =
                        content.parse_terminated(<LitStr as syn::parse::Parse>::parse, syn::Token![,])?;
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
                return syn::Error::new_spanned(variant, "only one #[str(default)] variant is allowed")
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

/// Generates the verbatim-identical `impl Query { all, bound, area, stats }`
/// block shared by fort-shaped scanner entities (gym, pokestop), which differ
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
/// macro never names a koji-scanner type directly (no crate dep, no cycle).
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
