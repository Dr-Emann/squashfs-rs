#![recursion_limit = "128"]

extern crate proc_macro;

use quote::{quote, quote_spanned};
use syn::parse_macro_input;

use proc_macro2::{Span, TokenStream};
use proc_macro_crate::crate_name;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{DeriveInput, Field, GenericParam, Generics};

fn get_crate_ident() -> TokenStream {
    let crate_name = crate_name("packed_serialize");
    let crate_name: &str = match &crate_name {
        Ok(name) => &name,
        Err(e) => {
            eprintln!("{}", e);
            "packed_serialize"
        }
    };
    let ident = syn::Ident::new(crate_name, Span::call_site());
    quote!(::#ident)
}

#[proc_macro_derive(PackedStruct)]
pub fn packed_struct(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;
    let mut generics = input.generics;
    let crate_ident = get_crate_ident();
    add_generic_bounds(&crate_ident, &mut generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    const DERIVE_TYPE_ERROR: &str = "#[derive(PackedStruct)] only works on structs";
    let fields = match input.data {
        syn::Data::Struct(ref data) => &data.fields,
        syn::Data::Enum(ref data) => {
            return syn::Error::new(data.enum_token.span(), DERIVE_TYPE_ERROR)
                .to_compile_error()
                .into()
        }
        syn::Data::Union(ref data) => {
            return syn::Error::new(data.union_token.span(), DERIVE_TYPE_ERROR)
                .to_compile_error()
                .into()
        }
    };

    let fields: Vec<&Field> = fields.iter().collect();
    let sizes: Vec<&syn::Type> = fields.iter().map(|field| &field.ty).collect();

    let size = calc_size_type(&sizes);
    let from_packed_impl = gen_from_packed(&crate_ident, &fields);

    let typenum_path = quote!( #crate_ident::generic_array::typenum );

    let offset_functions: TokenStream = (0..sizes.len())
        .map(|i| {
            let field_name = fields[i]
                .ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| i.to_string());

            let const_name = quote::format_ident!("OFFSET_{}", field_name.to_uppercase());
            let fn_name = quote::format_ident!("offset_{}", field_name);

            let size = calc_size_type(&sizes[..i]);

            quote!(
                pub const #const_name: usize = <#size as #typenum_path::Unsigned>::USIZE;

                #[inline]
                pub fn #fn_name() -> usize {
                    Self::#const_name
                }
            )
        })
        .collect();

    quote!(
        impl #impl_generics #crate_ident::PackedStruct for #ident #ty_generics #where_clause {
            type Size = #size;

            #from_packed_impl
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            #offset_functions
        }
    )
    .into()
    // TokenStream::new().into()
}

fn add_generic_bounds(crate_ident: &TokenStream, generics: &mut Generics) {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(syn::parse2(crate_ident.clone()).unwrap());
        }
    }
}

fn calc_size_type(types: &[&syn::Type]) -> TokenStream {
    let crate_ident = get_crate_ident();
    let typenum_path = quote!( #crate_ident::generic_array::typenum );
    let trait_path = quote!( #crate_ident::PackedStruct );

    fn calc_size_type_rec(
        types: &[&syn::Type],
        typenum_path: &TokenStream,
        trait_path: &TokenStream,
    ) -> TokenStream {
        if types.is_empty() {
            return quote!(#typenum_path::U0);
        }
        if types.len() == 1 {
            let ty = types[0];
            return quote_spanned!(ty.span()=> <#ty as #trait_path>::Size);
        }
        let (left, right) = types.split_at(types.len() / 2);
        let left = calc_size_type_rec(left, typenum_path, trait_path);
        let right = calc_size_type_rec(right, typenum_path, trait_path);
        quote!(#typenum_path::Sum<#left, #right>)
    }

    calc_size_type_rec(types, &typenum_path, &trait_path)
}

#[derive(Debug)]
struct FieldArrayInfo<'a> {
    field: &'a Field,
    field_name: TokenStream,
    sub_array_ptr_expr: TokenStream,
}

impl<'a> FieldArrayInfo<'a> {
    fn set_field(&self, crate_ident: &TokenStream) -> TokenStream {
        let name = &self.field_name;
        let ty = &self.field.ty;
        let ptr = &self.sub_array_ptr_expr;
        let expr = quote!( <#ty as #crate_ident::PackedStruct>::from_packed(unsafe { &*(#ptr) }) );
        quote!( #name : #expr, )
    }

    fn read_into_field(&self, crate_ident: &TokenStream) -> TokenStream {
        let name = &self.field_name;
        let ty = &self.field.ty;
        let ptr = &self.sub_array_ptr_expr;
        quote!( <#ty as #crate_ident::PackedStruct>::read_packed_arr(&mut self.#name, unsafe { &*(#ptr) }); )
    }

    fn write_from_field(&self, crate_ident: &TokenStream) -> TokenStream {
        let name = &self.field_name;
        let ty = &self.field.ty;
        let ptr = &self.sub_array_ptr_expr;

        quote!( <#ty as #crate_ident::PackedStruct>::write_packed_arr(&self.#name, unsafe { &mut *(#ptr) }); )
    }
}

fn field_subarrays<'a>(
    crate_ident: &TokenStream,
    fields: &[&'a Field],
    array_ident: &syn::Ident,
) -> Vec<FieldArrayInfo<'a>> {
    let mut field_sizes = Vec::with_capacity(fields.len());
    let mut field_number = 0;
    fields.iter().map(|field| {
        let sum_previous = calc_size_type(&field_sizes);
        let field_ty = &field.ty;
        field_sizes.push(&field.ty);
        let field_name = field.ident.clone().map(|i| i.into_token_stream()).unwrap_or_else(|| {
            let name = syn::Index::from(field_number);
            field_number += 1;
            name.into_token_stream()
        });
        FieldArrayInfo {
            field,
            field_name,
            sub_array_ptr_expr: quote!( &#array_ident[
                <#sum_previous as #crate_ident::generic_array::typenum::Unsigned>::to_usize()]
                    as *const u8
                    as *mut u8
                    as *mut #crate_ident::generic_array::GenericArray<u8, <#field_ty as #crate_ident::PackedStruct>::Size>
            )
        }
    }).collect()
}

fn gen_from_packed(crate_ident: &TokenStream, fields: &[&Field]) -> TokenStream {
    let array_ident = syn::Ident::new("array", Span::call_site());
    let subarrays = field_subarrays(crate_ident, fields, &array_ident);
    let from_packed_impls = subarrays.iter().map(|info| info.set_field(crate_ident));
    let read_packed_impls = subarrays
        .iter()
        .map(|info| info.read_into_field(crate_ident));
    let write_packed_impls = subarrays
        .iter()
        .map(|info| info.write_from_field(crate_ident));
    quote!(
        #[inline]
        fn from_packed(#array_ident: &#crate_ident::generic_array::GenericArray<u8, Self::Size>) -> Self
        where
            Self: Sized,
        {
            Self {
                #( #from_packed_impls )*
            }
        }

        #[inline]
        fn read_packed_arr(&mut self, #array_ident: &#crate_ident::generic_array::GenericArray<u8, Self::Size>) {
            #( #read_packed_impls )*
        }

        #[inline]
        fn write_packed_arr(&self, #array_ident: &mut #crate_ident::generic_array::GenericArray<u8, Self::Size>) {
            #( #write_packed_impls )*
        }
    )
}
