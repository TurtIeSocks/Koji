use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, ItemFn, LitStr, parse_macro_input};

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
