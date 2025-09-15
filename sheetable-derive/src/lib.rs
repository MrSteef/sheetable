//! # sheetable-derive
//!
//! Derives for the [`sheetable`] core traits.
//!
//! - `#[derive(Sheetable)]`: implement row mapping for **writable** columns, and
//!   return a **hydrated** instance from `from_values` (including calculated columns).
//! - `#[derive(SheetableReadOnly)]`: implement decoding for a **read-only**
//!   (calculated) details struct.
//!
//! ## Attributes
//!
//! ### `#[column("A")]`
//! Marks a field mapping to a spreadsheet column (A1 letters). Multi-letter
//! columns like `"AA"` are supported.
//!
//! ### `#[calculated(DetailsType)]`
//! On `#[derive(Sheetable)]` only, marks the single **details** field. The field
//! type must be a single generic parameter of the struct (e.g. `RO`), and
//! `DetailsType` must implement `sheetable::SheetableReadOnly`.
//!
//! ## Rules
//! - In `#[derive(Sheetable)]`, every field must have **exactly one** of
//!   `#[column("...")]` or `#[calculated(Type)]`. At most **one** field may be
//!   `#[calculated(..)]`.
//! - In `#[derive(SheetableReadOnly)]`, every field must have `#[column("...")]`.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput, Expr, ExprLit,
    Fields, GenericParam, Ident, Lit, LitStr, Meta, MetaNameValue, Type, TypePath,
};

/* -------------------------------------------------------------------------- */
/*                                     API                                    */
/* -------------------------------------------------------------------------- */

#[proc_macro_derive(Sheetable, attributes(column, calculated))]
pub fn derive_sheetable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_sheetable(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(SheetableReadOnly, attributes(column))]
pub fn derive_sheetable_read_only(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_read_only(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Shared                                   */
/* -------------------------------------------------------------------------- */

#[derive(Clone)]
struct ColumnField {
    ident: Ident,
    ty: Type,
    col_index: usize,
    field_name_lit: LitStr,
}

#[derive(Clone)]
struct CalculatedField {
    details_ty: Type,
    ident: Ident,
    generic_param_ident: Ident,
}

struct ParsedStruct<'a> {
    input: &'a DeriveInput,
    columns: Vec<ColumnField>,
    calculated: Option<CalculatedField>,
}

fn parse_struct(input: &DeriveInput, allow_calculated: bool) -> syn::Result<ParsedStruct<'_>> {
    let Data::Struct(DataStruct { fields: Fields::Named(named), .. }) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "this derive supports only structs with named fields",
        ));
    };

    let mut columns = Vec::<ColumnField>::new();
    let mut calculated: Option<CalculatedField> = None;

    for f in &named.named {
        let Some(ident) = f.ident.clone() else { continue; };
        let field_name_lit = LitStr::new(&ident.to_string(), ident.span());

        let mut col_attr: Option<usize> = None;
        let mut calc_attr: Option<Type> = None;

        for attr in &f.attrs {
            if let Some(idx) = parse_column_attr(attr)? {
                if col_attr.is_some() {
                    return Err(syn::Error::new(attr.span(), "duplicate #[column(\"...\")] on the same field"));
                }
                col_attr = Some(idx);
            }
            if allow_calculated {
                if let Some(ty) = parse_calculated_attr(attr)? {
                    if calc_attr.is_some() {
                        return Err(syn::Error::new(attr.span(), "duplicate #[calculated(Type)] on the same field"));
                    }
                    calc_attr = Some(ty);
                }
            }
        }

        if allow_calculated {
            let has_col = col_attr.is_some();
            let has_calc = calc_attr.is_some();
            if has_col == has_calc {
                return Err(syn::Error::new(
                    f.span(),
                    "each field must have exactly one of #[column(\"...\")] OR #[calculated(Type)]",
                ));
            }
        } else if col_attr.is_none() {
            return Err(syn::Error::new(
                f.span(),
                "fields must be marked with #[column(\"...\")]",
            ));
        }

        if let Some(idx) = col_attr {
            columns.push(ColumnField { ident, ty: f.ty.clone(), col_index: idx, field_name_lit });
        } else if let Some(details_ty) = calc_attr {
            let Some(generic_param_ident) = extract_single_generic_param_ident(&f.ty) else {
                return Err(syn::Error::new(
                    f.ty.span(),
                    "the type of a #[calculated(..)] field must be a single generic parameter (e.g., `RO`)",
                ));
            };
            if calculated.is_some() {
                return Err(syn::Error::new(
                    f.span(),
                    "only one #[calculated(..)] field is allowed per struct",
                ));
            }
            calculated = Some(CalculatedField { details_ty, ident, generic_param_ident });
        }
    }

    Ok(ParsedStruct { input, columns, calculated })
}

fn parse_column_attr(attr: &Attribute) -> syn::Result<Option<usize>> {
    if !attr.path().is_ident("column") {
        return Ok(None);
    }

    if let Ok(lit) = attr.parse_args::<LitStr>() {
        let s = lit.value();
        let idx = a1_col_to_index(&s).ok_or_else(|| {
            syn::Error::new(attr.span(), "invalid column string; use letters like \"A\" or \"AA\"")
        })?;
        return Ok(Some(idx));
    }

    if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta {
        if let Expr::Lit(ExprLit { lit: Lit::Str(ls), .. }) = value {
            let s = ls.value();
            let idx = a1_col_to_index(&s).ok_or_else(|| {
                syn::Error::new(attr.span(), "invalid column string; use letters like \"A\" or \"AA\"")
            })?;
            return Ok(Some(idx));
        }
    }

    Err(syn::Error::new(
        attr.span(),
        "#[column(\"A\")] or #[column = \"A\"] expected",
    ))
}

fn parse_calculated_attr(attr: &Attribute) -> syn::Result<Option<Type>> {
    if !attr.path().is_ident("calculated") {
        return Ok(None);
    }
    let ty: Type = attr
        .parse_args()
        .map_err(|_| syn::Error::new(attr.span(), "#[calculated(Type)] expects a single type"))?;
    Ok(Some(ty))
}

fn a1_col_to_index(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut n: usize = 0;
    for ch in s.chars() {
        let u = ch.to_ascii_uppercase();
        if !('A'..='Z').contains(&u) {
            return None;
        }
        let v = (u as u8 - b'A' + 1) as usize;
        n = n * 26 + v;
    }
    Some(n - 1)
}

fn extract_single_generic_param_ident(ty: &Type) -> Option<Ident> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        if path.segments.len() == 1 {
            let seg = &path.segments[0];
            if seg.arguments.is_empty() {
                return Some(seg.ident.clone());
            }
        }
    }
    None
}

// NEW: robust where-clause combiner
fn combine_where_clause(
    where_clause: Option<&syn::WhereClause>,
    extra_bounds: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    match (where_clause, extra_bounds.is_empty()) {
        (Some(wc), true) => wc.to_token_stream(),
        (Some(wc), false) => {
            let mut wc_tokens = wc.to_token_stream();
            wc_tokens.extend(quote! { , #(#extra_bounds),* });
            wc_tokens
        }
        (None, true) => quote! {},
        (None, false) => quote! { where #(#extra_bounds),* },
    }
}

/* -------------------------------------------------------------------------- */
/*                               Sheetable derive                              */
/* -------------------------------------------------------------------------- */

fn expand_sheetable(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let parsed = parse_struct(input, true)?;
    let struct_ident = &parsed.input.ident;
    let generics = parsed.input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Associated types
    let (read_only_ty, hydrated_ty) = if let Some(CalculatedField {
        details_ty, generic_param_ident, ..
    }) = &parsed.calculated {
        let mut applied_args = Vec::<proc_macro2::TokenStream>::new();
        for gp in generics.params.iter() {
            match gp {
                GenericParam::Type(tp) if tp.ident == *generic_param_ident => {
                    applied_args.push(details_ty.to_token_stream());
                }
                GenericParam::Type(tp) => applied_args.push(tp.ident.to_token_stream()),
                GenericParam::Lifetime(lt) => applied_args.push(lt.lifetime.to_token_stream()),
                GenericParam::Const(c) => applied_args.push(c.ident.to_token_stream()),
            }
        }
        let hydrated = if applied_args.is_empty() {
            quote! { #struct_ident }
        } else {
            quote! { #struct_ident<#(#applied_args),*> }
        };
        (details_ty.to_token_stream(), hydrated)
    } else {
        (quote! { () }, quote! { #struct_ident #ty_generics })
    };

    // Bounds
    let mut extra_bounds = Vec::<proc_macro2::TokenStream>::new();
    for c in &parsed.columns {
        let ty = &c.ty;
        extra_bounds.push(quote! { #ty: ::sheetable::EncodeCell + ::sheetable::DecodeCell });
    }
    if let Some(CalculatedField { details_ty, .. }) = &parsed.calculated {
        extra_bounds.push(quote! { #details_ty: ::sheetable::SheetableReadOnly });
    }
    let where_clause_combined = combine_where_clause(where_clause, &extra_bounds);

    // to_values
    let encoders = parsed.columns.iter().map(|c| {
        let ColumnField { ident, field_name_lit, .. } = c;
        quote! {
            out.push(
                ::sheetable::EncodeCell::encode_cell(&self.#ident)
                    .map_err(|e| ::sheetable::SheetError::encode(#field_name_lit, e))?
            );
        }
    });

    // from_values
    let decoders = parsed.columns.iter().map(|c| {
        let ColumnField { ident, ty, col_index, field_name_lit } = c;
        let idx = *col_index;
        quote! {
            let #ident: #ty = {
                let cell = values.get(#idx).ok_or(::sheetable::SheetError::missing(#idx))?;
                <#ty as ::sheetable::DecodeCell>::decode_cell(cell)
                    .map_err(|e| ::sheetable::SheetError::decode(#field_name_lit, e))?
            };
        }
    });

    let construct_tokens = if let Some(CalculatedField { details_ty, ident: details_ident, .. }) =
        &parsed.calculated
    {
        let init_writable = parsed.columns.iter().map(|c| {
            let id = &c.ident; quote! { #id }
        });
        quote! {
            let #details_ident: #details_ty =
                <#details_ty as ::sheetable::SheetableReadOnly>::from_values(values)?;
            Ok(Self::Hydrated { #(#init_writable),*, #details_ident })
        }
    } else {
        let init_writable = parsed.columns.iter().map(|c| {
            let id = &c.ident; quote! { #id }
        });
        quote! { Ok(Self::Hydrated { #(#init_writable),* }) }
    };

    Ok(quote! {
        impl #impl_generics ::sheetable::Sheetable for #struct_ident #ty_generics
            #where_clause_combined
        {
            type ReadOnly = #read_only_ty;
            type Hydrated  = #hydrated_ty;

            fn to_values(&self) -> Result<Vec<::serde_json::Value>, ::sheetable::SheetError> {
                let mut out = Vec::new();
                #(#encoders)*
                Ok(out)
            }

            fn from_values(values: &[::serde_json::Value]) -> Result<Self::Hydrated, ::sheetable::SheetError> {
                #(#decoders)*
                #construct_tokens
            }
        }
    })
}

/* -------------------------------------------------------------------------- */
/*                          SheetableReadOnly derive                           */
/* -------------------------------------------------------------------------- */

fn expand_read_only(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let parsed = parse_struct(input, false)?;
    let struct_ident = &parsed.input.ident;
    let generics = parsed.input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Bounds: DecodeCell for each field
    let mut extra_bounds = Vec::<proc_macro2::TokenStream>::new();
    for c in &parsed.columns {
        let ty = &c.ty;
        extra_bounds.push(quote! { #ty: ::sheetable::DecodeCell });
    }
    let where_clause_combined = combine_where_clause(where_clause, &extra_bounds);

    // Decoders
    let decoders = parsed.columns.iter().map(|c| {
        let ColumnField { ident, ty, col_index, field_name_lit } = c;
        let idx = *col_index;
        quote! {
            let #ident: #ty = {
                let cell = values.get(#idx).ok_or(::sheetable::SheetError::missing(#idx))?;
                <#ty as ::sheetable::DecodeCell>::decode_cell(cell)
                    .map_err(|e| ::sheetable::SheetError::decode(#field_name_lit, e))?
            };
        }
    });

    let initializers = parsed.columns.iter().map(|c| {
        let id = &c.ident; quote! { #id }
    });

    Ok(quote! {
        impl #impl_generics ::sheetable::SheetableReadOnly for #struct_ident #ty_generics
            #where_clause_combined
        {
            fn from_values(values: &[::serde_json::Value]) -> Result<Self, ::sheetable::SheetError> {
                #(#decoders)*
                Ok(Self { #(#initializers),* })
            }
        }
    })
}
