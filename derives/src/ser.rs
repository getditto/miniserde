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
            "currently only enums or structs with named fields are supported",
        )),
    }
}

fn derive_struct(input: &DeriveInput, fields: &FieldsNamed) -> Result<TokenStream> {
    let c = crate::frontend();

    let ident = &input.ident;
    let dummy = Ident::new(&format!("_IMPL_SERIALIZE_FOR_{}", ident), Span::call_site());

    let each_fieldname = &fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let each_fieldstr = fields
        .named
        .iter()
        .map(attr::name_of_field)
        .collect::<Result<Vec<_>>>()?;
    let each_idx = 0usize..;

    let bound = parse_quote!(#c::Serialize);
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);

    let n = fields.named.len();
    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl #impl_generics #c::Serialize for #ident #ty_generics #bounded_where_clause {
                fn view(&self) -> #c::ser::ValueView<'_> {
                    #c::ser::ValueView::Map(#c::__::Box::new({
                        (0 .. #n).map(move |i| match i {
                            #(
                                #each_idx => (
                                    &#each_fieldstr as &dyn #c::Serialize,
                                    &self.#each_fieldname as &dyn #c::Serialize,
                                ),
                            )*
                            _ => #c::__::std::unreachable!(),
                        })
                    }))
                }
            }
        };
    })
}

fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    use attr::EnumTaggingMode;

    let c = crate::frontend();

    let tagging_mode = EnumTaggingMode::from_attrs(&input.attrs)?;

    let Enum = &input.ident;
    let (intro_generics, fwd_generics, _) = input.generics.split_for_impl();
    let bound = parse_quote!(#c::Serialize);
    let where_clause = bound::where_clause_with_bound(&input.generics, bound);
    let dummy = Ident::new(&format!("_IMPL_SERIALIZE_FOR_{}", Enum), Span::call_site());

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
                        #c::ser::ValueView::Str(#c::__::Cow::Borrowed(#each_name))
                    }
                )*
            }
        )
    } else {
        // Non-trivial enum case:
        let match_arms = enumeration.variants.iter().map(|variant| Ok({
            let Variant = &variant.ident;
            let Variant_str = attr::name_of_variant(variant)?;
            let mut each_binding_str = vec![];
            let (pattern, each_binding) = match variant.fields {
                Fields::Named(FieldsNamed { ref named, .. }) => {
                    let each_binding =
                        named
                            .iter()
                            .map(|it| it.ident.as_ref().unwrap().clone())
                            .collect::<Vec<Ident>>()
                    ;
                    each_binding_str =
                        named
                            .iter()
                            .map(attr::name_of_field)
                            .collect::<Result<_>>()?
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
                    (pattern, bindings)
                },
            };

            match tagging_mode {
                | EnumTaggingMode::ExternallyTagged => {
                    // We have to be able to yield `&'view dyn Serialize` map
                    // representation of the fields of the current variant. In a
                    // newtyped-variant case, this is easy:
                    // ```rust
                    // match *self {
                    //     #Enum::#Variant(ref inner) => {
                    //         …
                    //         yield inner as &dyn Serialize
                    //     }
                    // }
                    // ```
                    // But when not having a newtype-variant, there is no `inner`
                    // direct pointer. So we'd like to be able to express something
                    // like:
                    // ```rust
                    //              "inner"
                    //       vvvvvvvvvvvvvvvvvvvvvvvv
                    // yield serialize_untagged(self) as &dyn Serialize
                    // ```
                    // And at this point, the `&dyn Serialize` output type requirement
                    // is *very* restrictive (≠ `impl 'view + Serialize`).
                    // So we are forced to use the `unsafe`-based transparent wrapper,
                    // as follows:
                    macro_rules! with_WrapEnum {( $($body:tt)* ) => (quote!({
                        #[repr(transparent)]
                        struct WrapEnum #intro_generics /* = */ (
                            #Enum #fwd_generics,
                        )
                        #where_clause
                        ;

                        impl #intro_generics
                            #c::Serialize
                        for
                            WrapEnum #fwd_generics
                        #where_clause
                        {
                            fn view (self: &'_ Self)
                              -> #c::ser::ValueView<'_>
                            {
                                match &self.0 {
                                    #Enum::#Variant { #pattern } => {
                                        $($body)*
                                    },
                                    _ => unsafe {
                                        /// Safety: the only way to obtain a reference
                                        /// to this kind of `WrapEnum` is through
                                        /// the following cast, which has had its variant
                                        /// already checked.
                                        extern {}
                                        #c::__::std::hint::unreachable_unchecked()
                                    },
                                }
                            }
                        }

                        unsafe {
                            /// # Safety
                            ///  - `WrapEnum` is a `#[repr(transparent)]` wrapper;
                            ///  - `WrapEnum` carries no safety invariants.
                            extern {}

                            #c::__::std::mem::transmute::<
                                &'__serde_view /* #Enum #fwd_generics */ Self,
                                &'__serde_view WrapEnum #fwd_generics,
                            >(self) as &dyn #c::Serialize
                        }
                    }))}

                    // Expr of type `&'view dyn Serialize`
                    let payload = match variant.fields {
                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() == 1
                        => quote!(
                            _0 as &dyn #c::Serialize
                        ),

                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() > 1
                        => with_WrapEnum! {
                            #c::ser::ValueView::Seq(#c::__::Box::new(
                                #c::__::std::iter::IntoIterator::into_iter(#c::__::vec![#(
                                    #each_binding as &dyn #c::Serialize,
                                )*])
                            ))
                        },

                        Fields::Unit | Fields::Unnamed(_) => quote!(
                            {
                                #[derive(#c::Serialize)] struct Empty;
                                &Empty {}
                            } as &'static dyn #c::Serialize
                        ),

                        Fields::Named(_) => with_WrapEnum! {
                            #c::ser::ValueView::Map(#c::__::Box::new(
                                #c::__::std::iter::IntoIterator::into_iter(#c::__::vec![#(
                                    (
                                        &#each_binding_str as &dyn #c::Serialize,
                                        #each_binding as &dyn #c::Serialize,
                                    ),
                                )*])
                            ))
                        },
                    };

                    quote!(
                        #Enum::#Variant { #pattern } => #c::ser::ValueView::Map(#c::__::Box::new(
                            #c::__::std::iter::once((
                                &#Variant_str as &dyn #c::Serialize,
                                #payload,
                            ))
                        )),
                    )
                },

                | EnumTaggingMode::Untagged => {
                    // Expr of type `ValueView<'view>`
                    let payload = match variant.fields {
                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() == 1
                        => quote!(
                            #c::Serialize::view(_0)
                        ),

                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() > 1
                        => quote!(
                            #c::ser::ValueView::Seq(#c::__::Box::new(
                                #c::__::std::iter::IntoIterator::into_iter(#c::__::vec![#(
                                    #each_binding as &dyn #c::Serialize,
                                )*])
                            ))
                        ),

                        Fields::Unit | Fields::Unnamed(_) => quote!(
                            #c::ser::ValueView::Null
                        ),

                        Fields::Named(_) => quote!(
                            #c::ser::ValueView::Map(#c::__::Box::new(
                                #c::__::std::iter::IntoIterator::into_iter(#c::__::vec![#(
                                    (
                                        &#each_binding_str as &dyn #c::Serialize,
                                        #each_binding as &dyn #c::Serialize,
                                    ),
                                )*])
                            ))
                        ),
                    };

                    quote!(
                        #Enum::#Variant { #pattern } => #payload,
                    )
                },

                | EnumTaggingMode::InternallyTagged { ref tag_name, content_name: None } => {
                    // Expr of type `impl 'v + Iterator<Item = (&'v dyn Serialize, &'v dyn Serialize)>`
                    let iterator = match variant.fields {
                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() > 1
                        => return Err(Error::new_spanned(
                            unnamed.iter().nth(1).unwrap(),
                            r#"`#[serde(tag = "…")]` cannot be used with non-newtype tuple variants"#,
                        )),

                        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                            if unnamed.len() == 1
                        => {
                            let ty = &unnamed.iter().next().unwrap().ty;
                            quote!(
                                match #c::Serialize::view(_0) {
                                    #c::ser::ValueView::Map(mut map) => {
                                        (0 .. map.remaining())
                                            .map(move |_| map.next().unwrap())
                                    },
                                    _ => #c::__::std::panic!(
                                        r#"The type `{}` cannot be used with `#[serde(tag = "…")]`"#,
                                        #c::__::std::any::type_name::<#ty>(),
                                    ),
                                }
                            )
                        },

                        Fields::Unit | Fields::Unnamed(_) => quote!(
                            #c::__::std::iter::empty()
                        ),

                        Fields::Named(_) => quote!(
                            #c::__::std::iter::IntoIterator::into_iter(#c::__::vec![
                                #(
                                    (
                                        &#each_binding_str as &dyn #c::Serialize,
                                        #each_binding as &dyn #c::Serialize,
                                    ),
                                )*
                            ])
                        ),
                    };
                    quote!(
                        #Enum::#Variant { #pattern } => #c::ser::ValueView::Map(#c::__::Box::new({
                            let mut iterator = #iterator;
                            (0 .. (iterator.len() + 1))
                                .map(move |i| if i > 0 {
                                    iterator.next().unwrap()
                                } else {
                                    (
                                        &#tag_name as &dyn #c::Serialize,
                                        &#Variant_str as &dyn #c::Serialize,
                                    )
                                })
                        })),
                    )
                },

                | EnumTaggingMode::InternallyTagged { content_name: Some(_), .. } => todo!(),
            }
        })).collect::<Result<Vec<_>>>()?;

        quote!(
            /// FIXME: do this in a more performant fashion
            extern {}
            match self { #(#match_arms)* }
        )
    };
    Ok(quote!(
        #[allow(non_upper_case_globals, unused_variables)]
        const #dummy: () = {
            impl #intro_generics
                #c::Serialize
            for
                #Enum #fwd_generics
            #where_clause
            {
                fn view<'__serde_view> (
                    self: &'__serde_view Self,
                ) -> #c::ser::ValueView<'__serde_view>
                {
                    #view_body
                }
            }
        };
    ))
}

fn derive_unit(input: &DeriveInput) -> Result<TokenStream> {
    let c = crate::frontend();

    let ident = &input.ident;
    let (intro_generics, fwd_generics, where_clause) = input.generics.split_for_impl();
    let dummy = Ident::new(&format!("_IMPL_SERIALIZE_FOR_{}", ident), Span::call_site());

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            impl #intro_generics
                #c::Serialize
            for
                #ident #fwd_generics #where_clause
            {
                fn view (self: &'_ Self)
                  -> #c::ser::ValueView<'_>
                {
                    #c::ser::ValueView::Null
                }
            }
        };
    })
}
