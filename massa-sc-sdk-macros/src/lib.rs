//! Procedural macros for `massa-sc-sdk`.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, FnArg, GenericParam, ItemFn, LitStr, Pat,
    ReturnType, Type,
};

/// Expose a function to the Massa runtime.
///
/// The function keeps its Rust signature (so unit tests call it directly) and
/// a `wasm32`-only C-ABI wrapper is generated that:
/// * reads the parameter buffer handed by the host into the single parameter,
///   which may be `Args`, `Vec<u8>` or `&[u8]` (or nothing),
/// * hands the return value back to the host. Supported return types:
///   nothing, `Vec<u8>`, `&[u8]`, `[u8; N]`, `Args`, `String`, `&str`,
///   `Address`, `bool`, integers (little-endian), `U256`, `Option<T>` (empty
///   when `None`) and `Result<T, E: Display>` (`Err` aborts with the message).
///
/// ```ignore
/// use massa_sc_sdk::prelude::*;
///
/// #[export]
/// pub fn transfer(mut args: Args) -> Result<(), ArgsError> {
///     let to: Address = args.next()?;
///     let amount: u64 = args.next()?;
///     storage::set_value(to.as_bytes(), &amount);
///     Ok(())
/// }
///
/// #[export(name = "balanceOf")]
/// pub fn balance_of(mut args: Args) -> u64 {
///     let who: Address = args.must_next("address");
///     storage::try_get_value(who.as_bytes()).unwrap_or(0)
/// }
/// # massa_sc_sdk::mock::reset();
/// # let me = context::caller();
/// # transfer(Args::new().add(&me).add(7u64)).unwrap();
/// # assert_eq!(balance_of(Args::new().add(&me)), 7);
/// ```
#[proc_macro_attribute]
pub fn export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let mut export_name = func.sig.ident.to_string();
    if !attr.is_empty() {
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("name") {
                let lit: LitStr = meta.value()?.parse()?;
                export_name = lit.value();
                Ok(())
            } else {
                Err(meta.error("expected `name = \"...\"`"))
            }
        });
        parse_macro_input!(attr with parser);
    }
    match expand_export(func, export_name) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_export(func: ItemFn, export_name: String) -> syn::Result<proc_macro2::TokenStream> {
    let sig = &func.sig;
    if sig.asyncness.is_some() || sig.unsafety.is_some() || !sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            sig.span(),
            "#[export] functions must be plain, safe, non-generic functions",
        ));
    }
    if sig.inputs.len() > 1 {
        return Err(syn::Error::new(
            sig.inputs.span(),
            "#[export] functions take at most one parameter (Args, Vec<u8> or &[u8])",
        ));
    }
    let ident = &sig.ident;
    let wrapper = format_ident!("__massa_export_{}", ident);
    let sdk = quote!(::massa_sc_sdk);

    let (wrapper_param, call) = match sig.inputs.first() {
        None => (quote!(), quote!(#ident())),
        Some(FnArg::Receiver(r)) => {
            return Err(syn::Error::new(r.span(), "#[export] does not support methods"))
        }
        Some(FnArg::Typed(pat)) => {
            if let Pat::Ident(p) = &*pat.pat {
                if p.subpat.is_some() {
                    return Err(syn::Error::new(pat.span(), "unsupported parameter pattern"));
                }
            }
            let arg = if matches!(&*pat.ty, Type::Reference(_)) {
                quote!(&__param)
            } else {
                quote!(#sdk::entrypoint::FromParam::from_param(__param))
            };
            (
                quote!(__arg: i32),
                quote! {{
                    let __param = #sdk::entrypoint::take_param(__arg);
                    #ident(#arg)
                }},
            )
        }
    };

    let returns_unit = match &sig.output {
        ReturnType::Default => true,
        ReturnType::Type(_, ty) => matches!(&**ty, Type::Tuple(t) if t.elems.is_empty()),
    };
    let body = if returns_unit {
        quote! { #call; }
    } else {
        quote! { #sdk::entrypoint::give_return(#call) }
    };
    let ret = if returns_unit { quote!() } else { quote!(-> i32) };

    let export_name_lit = LitStr::new(&export_name, Span::call_site());
    Ok(quote! {
        #func

        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        #[export_name = #export_name_lit]
        pub extern "C" fn #wrapper(#wrapper_param) #ret {
            #body
        }
    })
}

/// Derive `ArgsEncode` + `ArgsDecode` (hence `Serializable`) for a struct:
/// fields are encoded in declaration order, without any length prefix, like
/// an AssemblyScript class implementing `Serializable`. The struct can then
/// be an [`Args`] element, an entry-point parameter or return value, and a
/// typed datastore / `PersistentMap` value.
#[proc_macro_derive(Serializable)]
pub fn derive_serializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_serializable(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_serializable(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let sdk = quote!(::massa_sc_sdk);
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(s) => &s.fields,
        _ => return Err(syn::Error::new(input.span(), "#[derive(Serializable)] only supports structs")),
    };

    let (encode_fields, decode_body) = match fields {
        Fields::Named(named) => {
            let idents: Vec<_> = named.named.iter().map(|f| f.ident.clone().unwrap()).collect();
            (
                quote! { #( #sdk::ArgsEncode::encode_into(&self.#idents, out); )* },
                quote! { Ok(Self { #( #idents: #sdk::ArgsDecode::decode(args)?, )* }) },
            )
        }
        Fields::Unnamed(unnamed) => {
            let idx: Vec<syn::Index> = (0..unnamed.unnamed.len()).map(syn::Index::from).collect();
            let decode = unnamed.unnamed.iter().map(|_| quote!(#sdk::ArgsDecode::decode(args)?));
            (
                quote! { #( #sdk::ArgsEncode::encode_into(&self.#idx, out); )* },
                quote! { Ok(Self( #( #decode, )* )) },
            )
        }
        Fields::Unit => (quote! {}, quote! { Ok(Self) }),
    };

    // Bound every type parameter by both traits.
    let mut generics = input.generics.clone();
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(t) = param {
            t.bounds.push(syn::parse_quote!(#sdk::ArgsEncode));
            t.bounds.push(syn::parse_quote!(#sdk::ArgsDecode));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #sdk::ArgsEncode for #name #ty_generics #where_clause {
            fn encode_into(&self, out: &mut #sdk::prelude::Vec<u8>) {
                #encode_fields
            }
        }
        impl #impl_generics #sdk::ArgsDecode for #name #ty_generics #where_clause {
            fn decode(args: &mut #sdk::Args) -> ::core::result::Result<Self, #sdk::ArgsError> {
                #decode_body
            }
        }
        // Storable as a typed datastore value / PersistentMap value.
        impl #impl_generics #sdk::storage::StorageValue for #name #ty_generics #where_clause {
            fn to_storage(&self) -> #sdk::prelude::Vec<u8> {
                #sdk::ArgsEncode::encode(self)
            }
            fn from_storage(bytes: &[u8]) -> ::core::option::Option<Self> {
                <Self as #sdk::ArgsDecode>::decode_bytes(bytes).ok()
            }
        }
        // Entry points may return the struct (serialized) or take it as their parameter.
        impl #impl_generics #sdk::entrypoint::IntoReturn for #name #ty_generics #where_clause {
            fn into_return_bytes(self) -> #sdk::prelude::Vec<u8> {
                #sdk::ArgsEncode::encode(&self)
            }
        }
        impl #impl_generics #sdk::entrypoint::FromParam for #name #ty_generics #where_clause {
            fn from_param(bytes: #sdk::prelude::Vec<u8>) -> Self {
                match #sdk::ArgsDecode::decode_bytes(&bytes) {
                    ::core::result::Result::Ok(v) => v,
                    ::core::result::Result::Err(e) => #sdk::abort(&#sdk::prelude::format!("invalid parameter: {e}")),
                }
            }
        }
    })
}
