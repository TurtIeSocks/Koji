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

    let message_str = message.map(|f| f.value())
        .unwrap_or(fn_sig.ident.to_string());

    // Generate the new function with timing code
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            let __timer_start = std::time::Instant::now();
            log::info!("starting {}", #message_str);

            // Original function body wrapped to capture return value
            let __timer_result = (|| #fn_block)();

            log::info!("finished {} in {:.2}s", #message_str, __timer_start.elapsed().as_secs_f32());

            __timer_result
        }
    };

    TokenStream::from(expanded)
}

/// Derives string conversions for a unit-variant enum:
/// `as_str`, `from_str_opt`, `Serialize` (as string), `Deserialize` (from
/// string), and `Display`. Each variant must carry `#[str("literal")]`.
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

    let mut as_str_arms = Vec::new();
    let mut from_arms = Vec::new();
    for variant in &data.variants {
        let vident = &variant.ident;
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(variant, "StrEnum variants must be unit variants")
                .to_compile_error()
                .into();
        }
        let mut lit: Option<LitStr> = None;
        for attr in &variant.attrs {
            if attr.path().is_ident("str") {
                match attr.parse_args::<LitStr>() {
                    Ok(s) => lit = Some(s),
                    Err(e) => return e.to_compile_error().into(),
                }
            }
        }
        let lit = match lit {
            Some(s) => s,
            None => {
                return syn::Error::new_spanned(variant, "missing #[str(\"...\")] attribute")
                    .to_compile_error()
                    .into();
            }
        };
        as_str_arms.push(quote! { #name::#vident => #lit });
        from_arms.push(quote! { #lit => ::core::option::Option::Some(#name::#vident) });
    }

    quote! {
        impl #name {
            pub const fn as_str(&self) -> &'static str {
                match self { #(#as_str_arms),* }
            }
            pub fn from_str_opt(s: &str) -> ::core::option::Option<Self> {
                match s { #(#from_arms,)* _ => ::core::option::Option::None }
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
                Self::from_str_opt(&s).ok_or_else(|| {
                    <D::Error as ::serde::de::Error>::custom(
                        ::std::format!("unknown {}: {}", ::core::stringify!(#name), s)
                    )
                })
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
