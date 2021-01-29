use ::core::ops::Not as _;
use ::proc_macro2::{Span, TokenStream};
use ::quote::{format_ident, quote, ToTokens};
use ::syn::{spanned::Spanned, Result, *};

use crate::{attr, bound};

pub fn derive(input: DeriveInput) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => derive_struct(&input, fields),
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => derive_struct(&input, &parse_quote!({})),
        Data::Enum(enumeration) => derive_enum(&input, enumeration),
        _ => Err(Error::new(
            Span::call_site(),
            "currently only structs with named fields are supported",
        )),
    }
}

pub fn derive_struct(input: &DeriveInput, fields: &FieldsNamed) -> Result<TokenStream> {
    let c = crate::frontend();

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let dummy = Ident::new(
        &format!("_IMPL_DESERIALIZE_FOR_{}", ident),
        Span::call_site(),
    );

    let skipped_fields = || {
        fields
            .named
            .iter()
            .filter(|f| attr::has_skip_deserializing(&f.attrs))
    };
    let non_skipped_fields = || {
        fields
            .named
            .iter()
            .filter(|f| attr::has_skip_deserializing(&f.attrs).not())
    };

    let each_skipped_field = skipped_fields().map(|f| &f.ident);
    let each_field = non_skipped_fields().map(|f| &f.ident).collect::<Vec<_>>();
    let EachFieldTy = non_skipped_fields().map(|f| &f.ty);
    let each_field_str = fields
        .named
        .iter()
        .map(attr::name_of_field)
        .collect::<Result<Vec<_>>>()?;

    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__a");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(#c::Deserialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);

    let mb_deserialize_null = if fields.named.is_empty() {
        Some(quote!(
            fn null(&mut self) -> #c::Result<()> {
                self.out = #c::__::Some(#ident {});
                #c::Result::Ok(())
            }
        ))
    } else {
        None
    };

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        const #dummy: () = {
            #[repr(C)]
            struct __Visitor #impl_generics #where_clause {
                out: #c::__::Option<#ident #ty_generics>,
            }

            impl #impl_generics #c::Deserialize for #ident #ty_generics #bounded_where_clause {
                fn begin(out: &'_ mut #c::__::Option<Self>) -> &'_ mut dyn #c::de::Visitor {
                    unsafe {
                        &mut *{
                            out
                            as *mut #c::__::Option<Self>
                            as *mut __Visitor #ty_generics
                        }
                    }
                }
            }

            impl #impl_generics #c::de::Visitor for __Visitor #ty_generics #bounded_where_clause {
                #mb_deserialize_null

                fn map(&mut self) -> #c::Result<#c::__::Box<dyn #c::de::Map + '_>> {
                    #c::__::Ok(#c::__::Box::new(__State {
                        #(
                            #each_field: #c::Deserialize::default(),
                        )*
                        out: &mut self.out,
                    }))
                }
            }

            struct __State #wrapper_impl_generics #where_clause {
                #(
                    #each_field: #c::__::Option<#EachFieldTy>,
                )*
                out: &'__a mut #c::__::Option<#ident #ty_generics>,
            }

            impl #wrapper_impl_generics #c::de::StrKeyMap for __State #wrapper_ty_generics #bounded_where_clause {
                fn key(&mut self, __k: &#c::__::str) -> #c::Result<&mut dyn #c::de::Visitor> {
                    match __k {
                        #(
                            #each_field_str => #c::__::Ok(#c::Deserialize::begin(&mut self.#each_field)),
                        )*
                        _ => #c::__::Ok(#c::de::Visitor::ignore()),
                    }
                }

                fn finish(self: #c::__::Box<Self>) -> #c::Result<()> {
                    #(
                        let #each_field = self.#each_field.ok_or(#c::Error)?;
                    )*
                    *self.out = #c::__::Some(#ident {
                        #(
                            #each_field,
                        )*
                        #(
                            #each_skipped_field: #c::__::Default::default(),
                        )*
                    });
                    #c::__::Ok(())
                }
            }
        };
    })
}

pub fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    use attr::EnumTaggingMode;
    let c = crate::frontend();

    let (intro_generics, fwd_generics, _) = input.generics.split_for_impl();
    let bound = parse_quote!(#c::Deserialize);
    let where_clause = bound::where_clause_with_bound(&input.generics, bound);
    let tagging_mode = EnumTaggingMode::from_attrs(&input.attrs)?;
    let Enum = &input.ident;

    let is_trivial_enum = enumeration
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit));
    let ret = if is_trivial_enum {
        let each_var_ident = enumeration
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
        let each_name = enumeration
            .variants
            .iter()
            .map(attr::name_of_variant)
            .collect::<Result<Vec<_>>>()?;

        quote!(
            impl #intro_generics
                #c::de::Visitor
            for
                __Visitor #fwd_generics
            {
                fn string (self: &'_ mut Self, s: &'_ #c::__::str)
                  -> #c::Result<()>
                {
                    let value = match s {
                        #( #each_name => #Enum::#each_var_ident, )*
                        _ => { return #c::__::Err(#c::Error) },
                    };
                    self.out = #c::__::Some(value);
                    #c::__::Ok(())
                }
            }
        )
    } else {
        let is_serde_attr = |attr: &'_ &'_ Attribute| attr.path.is_ident("serde");
        let serde_enum_attrs = input.attrs.iter().filter(is_serde_attr);

        let generics_map = bound::with_lifetime_bound(&input.generics, "'__serde_de_map");
        let (intro_generics_map, fwd_generics_map, _) = generics_map.split_for_impl();

        let all_variants_are_newtypes = enumeration.variants.iter().all(|variant| {
            matches!(
                variant.fields,
                Fields::Unnamed(FieldsUnnamed { ref unnamed, .. })
                if unnamed.len() == 1
            )
        });

        let mut define_helper_enum = quote!();

        let map = if all_variants_are_newtypes {
            // Case where all the enum variants are of the form:
            // `Variant(VariantTy) where VariantTy : Deserialize`
            let EachVariant = enumeration
                .variants
                .iter()
                .map(|v| &v.ident)
                .collect::<Vec<_>>();
            let EachVariant_str = enumeration
                .variants
                .iter()
                .map(attr::name_of_variant)
                .collect::<Result<Vec<_>>>()?;
            let EachVariantTy = enumeration.variants.iter().map(|v| match v.fields {
                Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => unnamed.first().unwrap(),
                _ => unreachable!(),
            });
            match tagging_mode {
                EnumTaggingMode::ExternallyTagged => quote!(
                    struct __Map #intro_generics_map
                    #where_clause
                    {
                        __serde_out: &'__serde_de_map mut #c::__::Option<
                            #Enum #fwd_generics,
                        >,
                        #(
                            #EachVariant: #c::__::Option<
                                #EachVariantTy,
                            >,
                        )*
                    }

                    impl #intro_generics_map
                        #c::de::StrKeyMap
                    for
                        __Map #fwd_generics_map
                    #where_clause
                    {
                        fn key (
                            self: &'_ mut Self,
                            key: &'_ str,
                        ) -> #c::Result<&'_ mut dyn #c::de::Visitor>
                        {
                            match key {
                            #(
                                #EachVariant_str => #c::Result::Ok(
                                    #c::de::Deserialize::begin(&mut self.#EachVariant)
                                ),
                            )*
                                _ => #c::Result::Err(#c::Error),
                            }
                        }

                        fn finish (self: #c::__::Box<Self>)
                          -> #c::Result<()>
                        {
                            #(
                                if let #c::__::Some(variant) = self.#EachVariant {
                                    let prev = self.__serde_out.replace(
                                        #Enum::#EachVariant(variant)
                                    );
                                    if prev.is_some() {
                                        return #c::Result::Err(#c::Error);
                                    }
                                }
                            )*
                            #c::Result::Ok(())
                        }
                    }

                    let map: __Map #fwd_generics_map = __Map {
                        __serde_out: &mut self.out,
                        #(
                            #EachVariant : #c::__::None,
                        )*
                    };
                    map
                ),
                _ => todo!("{:?}", tagging_mode),
            }
        } else {
            // case `all_variants_are_newtypes.not()`.
            // Use a helper enum to go back to an `all_variants_are_newtypes`
            // case, and delegate to it.
            let __Helper_Enum = format_ident!("__Helper_{}", Enum);
            let mut helper_variants = enumeration.variants.clone();
            let mut impl_into_branches = Vec::with_capacity(helper_variants.len());
            helper_variants.iter_mut().for_each(|variant| {
                variant.attrs.retain(|a| is_serde_attr(&a));
                let Variant = &variant.ident;
                variant.fields = Fields::Unnamed(match variant.fields {
                    Fields::Named(FieldsNamed {
                        named: ref each_field_def,
                        ..
                    }) => {
                        let each_field = each_field_def
                            .iter()
                            .map(|it| &it.ident)
                            .collect::<Vec<_>>();
                        let each_field_def = each_field_def.iter();
                        let __Helper_Variant = format_ident!("__Helper_{}", Variant);
                        let PhantomData = {
                            let each_lifetime = input.generics.lifetimes();
                            let each_type_param = input.generics.type_params().map(|tp| &tp.ident);
                            quote!(
                                #c::__::std::marker::PhantomData<(
                                    #(
                                        & #each_lifetime (),
                                    )*
                                    #(
                                        #each_type_param,
                                    )*
                                )>
                            )
                        };
                        define_helper_enum.extend(quote! {
                            #[derive(#c::Deserialize)]
                            struct #__Helper_Variant #intro_generics
                            #where_clause
                            {
                                #(
                                    #each_field_def,
                                )*
                                #[serde(skip)]
                                __serde_generic_params: #PhantomData,
                            }
                        });
                        impl_into_branches.push(quote!(
                            #__Helper_Enum::#Variant(#__Helper_Variant {
                                #( #each_field, )* ..
                            }) => #Enum::#Variant {
                                #( #each_field, )*
                            }
                        ));
                        parse_quote!((
                            #__Helper_Variant #fwd_generics,
                        ))
                    }

                    Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) if unnamed.len() == 1 => {
                        impl_into_branches.push(quote!(
                            #__Helper_Enum::#Variant(it) => #Enum::#Variant(it)
                        ));
                        let Ty = unnamed.iter().next().unwrap().ty.to_token_stream();
                        parse_quote!((
                            #Ty,
                        ))
                    }

                    Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) if unnamed.len() > 1 => {
                        let each_idx = unnamed
                            .iter()
                            .zip(0..)
                            .map(|(f, index)| Index {
                                index,
                                span: f.span(),
                            })
                            .collect::<Vec<_>>();
                        impl_into_branches.push(quote!(
                            #__Helper_Enum :: #Variant (
                                ( #(#each_idx,)* )
                            ) => #Enum::#Variant(
                                #(#each_idx),*
                            )
                        ));
                        let each_ty = unnamed.iter().map(|field| &field.ty);
                        parse_quote!((
                            (#(#each_ty),*),
                        ))
                    }

                    Fields::Unit | Fields::Unnamed(_) => {
                        impl_into_branches.push(quote!(
                            #__Helper_Enum::#Variant {} => #Enum::#Variant {}
                        ));
                        parse_quote!((
                            #c::__::Empty,
                        ))
                    }
                });
            });
            let each_Helper_Enum_variant = helper_variants.iter();
            define_helper_enum.extend(quote!(
                #[derive(#c::Deserialize)]
                #( #serde_enum_attrs )*
                enum #__Helper_Enum #intro_generics
                #where_clause
                {
                    #(
                        #each_Helper_Enum_variant,
                    )*
                }
                impl #intro_generics
                    #__Helper_Enum #fwd_generics
                #where_clause
                {
                    #[inline]
                    fn into (self: Self) -> #Enum #fwd_generics
                    {
                        match self {
                            #( #impl_into_branches, )*
                        }
                    }
                }
            ));
            quote!(
                impl #intro_generics_map
                    #c::de::Map
                for
                    __Map #fwd_generics_map
                #where_clause
                {
                    fn val_with_key (
                        self: &'_ mut Self,
                        with_key: &'_ mut dyn #c::__::FnMut(
                            #c::Result<&'_ mut dyn #c::de::Visitor>
                        ) -> #c::Result<()>,
                    ) -> #c::Result<&'_ mut dyn #c::de::Visitor>
                    {
                        #c::de::Map::val_with_key(
                            &mut *self.helper_map,
                            with_key,
                        )
                    }

                    fn finish (self: #c::__::Box<Self>)
                      -> #c::Result<()>
                    {
                        #c::de::Map::finish(self.helper_map)?;
                        let helper = *#c::__::AliasedBox::assume_unique(self.helper_heap_slot);
                        *self.out = #c::__::Some(helper.ok_or(#c::Error)?.into());
                        #c::Result::Ok(())
                    }
                }

                struct __Map #intro_generics_map
                #where_clause
                {
                    out: &'__serde_de_map mut #c::__::Option<
                        #Enum #fwd_generics,
                    >,
                    helper_heap_slot: #c::__::AliasedBox<#c::__::Option<
                        #__Helper_Enum #fwd_generics,
                    >>,
                    helper_map: #c::__::Box<dyn #c::de::Map + '__serde_de_map>,
                }

                let mut helper_heap_slot = #c::__::AliasedBox::from(#c::__::Box::new(#c::__::None));

                let map: __Map #fwd_generics_map = __Map {
                    out: &mut self.out,
                    helper_map: unsafe {
                        #c::Deserialize::begin(&mut *helper_heap_slot.ptr())
                            .map()?
                    },
                    helper_heap_slot,
                };
                map
            )
        };
        quote!(
            #define_helper_enum

            impl #intro_generics
                #c::de::Visitor
            for
                __Visitor #fwd_generics
            #where_clause
            {
                fn map<'__serde_de_map> (self: &'__serde_de_map mut Self)
                  -> #c::Result<#c::__::Box<dyn #c::de::Map + '__serde_de_map>>
                {
                    #c::Result::Ok(#c::__::Box::new({
                        #map
                    }) as #c::__::Box<dyn #c::de::Map + '__serde_de_map>)
                }
            }
        )
    };

    let dummy = Ident::new(
        &format!("_IMPL_DESERIALIZE_FOR_{}", Enum),
        Span::call_site(),
    );
    Ok(quote!(
        #[allow(non_upper_case_globals, nonstandard_style, unused_variables)]
        const #dummy: () = {
            #[repr(C)]
            struct __Visitor #intro_generics
            #where_clause
            {
                out: #c::__::Option<#Enum #fwd_generics>,
            }

            impl #intro_generics
                #c::Deserialize
            for
                #Enum #fwd_generics
            #where_clause
            {
                fn begin (out: &'_ mut #c::__::Option<Self>)
                  -> &'_ mut dyn #c::de::Visitor
                {
                    unsafe {
                        &mut *{
                            out
                            as *mut #c::__::Option<Self>
                            as *mut __Visitor #fwd_generics
                        }
                    }
                }
            }

            #ret
        };
    ))
}
