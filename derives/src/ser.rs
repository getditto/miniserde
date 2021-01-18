use crate::{attr, bound};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, Data, DataEnum, DataStruct, DeriveInput, Error, Fields, FieldsNamed, Ident, Result,
};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => derive_struct(&input, &fields),
        Data::Enum(enumeration) => derive_enum(&input, enumeration),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

fn derive_struct(input: &DeriveInput, fields: &FieldsNamed) -> Result<TokenStream> {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let dummy = Ident::new(
        &format!("_IMPL_MINISERIALIZE_FOR_{}", ident),
        Span::call_site(),
    );

    let fieldname = &fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let fieldstr = fields
        .named
        .iter()
        .map(attr::name_of_field)
        .collect::<Result<Vec<_>>>()?;
    let index = 0usize..;

    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__a");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(miniserde_ditto::Serialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl #impl_generics miniserde_ditto::Serialize for #ident #ty_generics #bounded_where_clause {
                fn begin(&self) -> miniserde_ditto::ser::ValueView<'_> {
                    miniserde_ditto::ser::ValueView::Map(miniserde_ditto::__private::Box::new(__Map {
                        data: self,
                        state: 0,
                    }))
                }
            }

            struct __Map #wrapper_impl_generics #where_clause {
                data: &'__a #ident #ty_generics,
                state: miniserde_ditto::__private::usize,
            }

            impl #wrapper_impl_generics miniserde_ditto::ser::Map<'__a> for __Map #wrapper_ty_generics #bounded_where_clause {
                fn next (self: &'_ mut Self)
                  -> miniserde_ditto::__private::Option<(
                        &'__a dyn miniserde_ditto::Serialize,
                        &'__a dyn miniserde_ditto::Serialize,
                    )>
                {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        #(
                            #index => miniserde_ditto::__private::Some((
                                &#fieldstr,
                                &self.data.#fieldname,
                            )),
                        )*
                        _ => miniserde_ditto::__private::None,
                    }
                }
                #[allow(nonstandard_style)]
                fn remaining(&self) -> usize
                {
                    0 #(+ { let #fieldname = 1; #fieldname })* - self.state
                }
            }
        };
    })
}

fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }

    let ident = &input.ident;
    let dummy = Ident::new(
        &format!("_IMPL_MINISERIALIZE_FOR_{}", ident),
        Span::call_site(),
    );

    let var_idents = enumeration
        .variants
        .iter()
        .map(|variant| match variant.fields {
            Fields::Unit => Ok(&variant.ident),
            _ => Err(Error::new_spanned(
                variant,
                "Invalid variant: only simple enum variants without fields are supported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let names = enumeration
        .variants
        .iter()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl miniserde_ditto::Serialize for #ident {
                fn begin(&self) -> miniserde_ditto::ser::ValueView<'_> {
                    match self {
                        #(
                            #ident::#var_idents => {
                                miniserde_ditto::ser::ValueView::Str(miniserde_ditto::__private::Cow::Borrowed(#names))
                            }
                        )*
                    }
                }
            }
        };
    })
}
