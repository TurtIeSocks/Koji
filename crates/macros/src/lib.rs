use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

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
        .and_then(|f| Some(f.value()))
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
