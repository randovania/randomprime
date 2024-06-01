include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn resource_info(item: TokenStream) -> TokenStream {
    let res_name = parse_macro_input!(item as LitStr);
    if let Some(res_data) = RESOURCES.get(&res_name.value()[..]) {
        syn::parse_str::<proc_macro2::TokenStream>(res_data)
            .unwrap()
            .into()
    } else {
        syn::Error::new_spanned(res_name, "Unknown resource")
            .to_compile_error()
            .into()
    }
}
