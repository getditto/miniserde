use crate::{attr, bound};
use ::proc_macro2::{Span, TokenStream};
use ::quote::{format_ident, quote};
use ::syn::{spanned::Spanned, Result, *};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => derive_struct(&input, &fields),
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => derive_unit(&input),
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
    let dummy = Ident::new(&format!("_IMPL_SERIALIZE_FOR_{}", ident), Span::call_site());

    let each_fieldname = &fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let each_fieldstr = fields
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
                fn view(&self) -> miniserde_ditto::ser::ValueView<'_> {
                    miniserde_ditto::ser::ValueView::Map(miniserde_ditto::__::Box::new(__Map {
                        data: self,
                        state: 0,
                    }))
                }
            }

            struct __Map #wrapper_impl_generics #where_clause {
                data: &'__a #ident #ty_generics,
                state: miniserde_ditto::__::usize,
            }

            impl #wrapper_impl_generics miniserde_ditto::ser::Map<'__a> for __Map #wrapper_ty_generics #bounded_where_clause {
                fn next (self: &'_ mut Self)
                  -> miniserde_ditto::__::Option<(
                        &'__a dyn miniserde_ditto::Serialize,
                        &'__a dyn miniserde_ditto::Serialize,
                    )>
                {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        #(
                            #index => miniserde_ditto::__::Some((
                                &#each_fieldstr,
                                &self.data.#each_fieldname,
                            )),
                        )*
                        _ => miniserde_ditto::__::None,
                    }
                }
                #[allow(nonstandard_style)]
                fn remaining(&self) -> usize
                {
                    0 #(+ { let #each_fieldname = 1; #each_fieldname })* - self.state
                }
            }
        };
    })
}
use attr::EnumTaggingMode;

#[allow(nonstandard_style)]
fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let tagging_mode = EnumTaggingMode::from_attrs(&input.attrs)?;
    // if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
    //     return Err(Error::new(
    //         Span::call_site(),
    //         "Enums with generics are not supported",
    //     ));
    // }

    let Enum = &input.ident;
    let (intro_generics, fwd_generics, _) = input.generics.split_for_impl();
    let bound = parse_quote!(miniserde_ditto::Serialize);
    let where_clause = bound::where_clause_with_bound(&input.generics, bound);
    let dummy = Ident::new(
        &format!("_IMPL_MINISERIALIZE_FOR_{}", Enum),
        Span::call_site(),
    );

    let is_trivial_enum = enumeration
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit));
    let view_body = if is_trivial_enum {
        let each_var_ident = enumeration
            .variants
            .iter()
            .map(|it| &it.ident)
            .collect::<Vec<_>>();
        let each_name = enumeration
            .variants
            .iter()
            .map(attr::name_of_variant)
            .collect::<Result<Vec<_>>>()?;

        quote!(
            match self {
                #(
                    #Enum::#each_var_ident => {
                        miniserde_ditto::ser::ValueView::Str(miniserde_ditto::__::Cow::Borrowed(#each_name))
                    }
                )*
            }
        )
    } else {
        // Non-trivial enum case:
        let match_arms = enumeration.variants.iter().map(|variant| {
            let Variant = &variant.ident;
            let (pattern, each_binding) = match variant.fields {
                Fields::Named(FieldsNamed { ref named, .. }) => {
                    let each_binding =
                        named
                            .iter()
                            .map(|it| it.ident.as_ref().unwrap().clone())
                            .collect::<Vec<Ident>>()
                    ;
                    (
                        quote!(
                            #( #each_binding ),*
                        ),
                        each_binding,
                    )
                },
                Fields::Unit => (
                    quote!( .. ),
                    vec![],
                ),
                Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
                    let mut bindings = vec![];
                    let pattern =
                        unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| {
                                let idx = Index { index: i as _, span: field.ty.span() };
                                let binding = format_ident!(
                                    "_{}", i, span = idx.span,
                                );
                                let ret = quote!(
                                    #idx : #binding,
                                );
                                bindings.push(binding);
                                ret
                            })
                            .collect::<TokenStream>()
                    ;
                    if true { todo!() } else { (pattern, bindings) }
                },
            };
            match tagging_mode {
                | EnumTaggingMode::ExternallyTagged => quote!(
                    #Enum::#Variant { .. } => miniserde_ditto::ser::ValueView::Map(miniserde_ditto::__::Box::new({
                        #[repr(transparent)]
                        struct WrapEnum #intro_generics /* = */ (
                            #Enum #fwd_generics,
                        )
                        #where_clause
                        ;

                        impl #intro_generics
                            miniserde_ditto::Serialize
                        for
                            WrapEnum #fwd_generics
                        #where_clause
                        {
                            fn view (self: &'_ Self)
                              -> miniserde_ditto::ser::ValueView<'_>
                            {
                                match &self.0 {
                                    #Enum::#Variant { #pattern } => {
                                        miniserde_ditto::ser::ValueView::Map(miniserde_ditto::__::Box::new(
                                            miniserde_ditto::__::std::iter::IntoIterator::into_iter(miniserde_ditto::__::vec![#(
                                                (
                                                    &miniserde_ditto::__::stringify!(#each_binding) as &dyn miniserde_ditto::ser::Serialize,
                                                    #each_binding as &dyn miniserde_ditto::ser::Serialize,
                                                ),
                                            )*])
                                        ))
                                    },
                                    _ => unsafe {
                                        /// Safety: the only way to obtain a reference
                                        /// to this kind of `WrapEnum` is through
                                        /// the following cast, which has had its variant
                                        /// already checked.
                                        miniserde_ditto::__::std::hint::unreachable_unchecked()
                                    },
                                }
                            }
                        }

                        miniserde_ditto::__::std::iter::once((
                            &miniserde_ditto::__::stringify!(#Variant) as &dyn miniserde_ditto::ser::Serialize,
                            unsafe {
                                /// # Safety
                                ///  - `WrapEnum` is a `#[repr(transparent)]` wrapper;
                                ///  - `WrapEnum` carries no safety invariants.
                                extern {}

                                miniserde_ditto::__::std::mem::transmute::<
                                    &'__serde_view #Enum #fwd_generics,
                                    &'__serde_view WrapEnum #fwd_generics,
                                >(self)
                            } as &dyn miniserde_ditto::ser::Serialize,
                        ))
                    })),
                ),

                | EnumTaggingMode::Untagged => quote!(
                    #Enum::#Variant { #pattern } => miniserde_ditto::ser::ValueView::Map(Box::new({
                        miniserde_ditto::__::std::iter::IntoIterator::into_iter(miniserde_ditto::__::vec![#(
                            (
                                &miniserde_ditto::__::stringify!(#each_binding) as &dyn miniserde_ditto::ser::Serialize,
                                #each_binding as &dyn miniserde_ditto::ser::Serialize,
                            ),
                        )*])
                    })),
                ),

                | EnumTaggingMode::InternallyTagged { ref tag_name, content_name: None } => quote!(
                    #Enum::#Variant { #pattern } => miniserde_ditto::ser::ValueView::Map(Box::new({
                        miniserde_ditto::__::std::iter::IntoIterator::into_iter(miniserde_ditto::__::std::vec![
                            (
                                &#tag_name as &dyn miniserde_ditto::ser::Serialize,
                                &miniserde_ditto::__::stringify!(#Variant) as &dyn miniserde_ditto::ser::Serialize,
                            ),
                            #(
                                (
                                    &miniserde_ditto::__::stringify!(#each_binding) as &dyn miniserde_ditto::ser::Serialize,
                                    #each_binding as &dyn miniserde_ditto::ser::Serialize,
                                ),
                            )*
                        ])
                    })),
                ),

                | EnumTaggingMode::InternallyTagged { .. } => todo!(),
            }
        });

        quote!(
            /// FIXME: do this in a more performant fashion
            extern {}
            match self { #(#match_arms)* }
        )
    };
    Ok(quote!(
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl #intro_generics
                miniserde_ditto::Serialize
            for
                #Enum #fwd_generics
            #where_clause
            {
                fn view<'__serde_view> (
                    self: &'__serde_view Self,
                ) -> miniserde_ditto::ser::ValueView<'__serde_view>
                {
                    #view_body
                }
            }
        };
    ))
}

fn derive_unit(input: &DeriveInput) -> Result<TokenStream> {
    let ident = &input.ident;
    let (intro_generics, fwd_generics, where_clause) = input.generics.split_for_impl();
    let dummy = Ident::new(&format!("_IMPL_SERIALIZE_FOR_{}", ident), Span::call_site());

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl #intro_generics
                miniserde_ditto::Serialize
            for
                #ident #fwd_generics #where_clause
            {
                fn view (self: &'_ Self)
                  -> miniserde_ditto::ser::ValueView<'_>
                {
                    miniserde_ditto::ser::ValueView::Null
                }
            }
        };
    })
}
