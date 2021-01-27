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

    let fieldname = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
    let fieldty = fields.named.iter().map(|f| &f.ty);
    let fieldstr = fields
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
                __out: #c::__::Option<#ident #ty_generics>,
            }

            impl #impl_generics #c::Deserialize for #ident #ty_generics #bounded_where_clause {
                fn begin(__out: &'_ mut #c::__::Option<Self>) -> &'_ mut dyn #c::de::Visitor {
                    unsafe {
                        &mut *{
                            __out
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
                            #fieldname: #c::Deserialize::default(),
                        )*
                        __out: &mut self.__out,
                    }))
                }
            }

            struct __State #wrapper_impl_generics #where_clause {
                #(
                    #fieldname: #c::__::Option<#fieldty>,
                )*
                __out: &'__a mut #c::__::Option<#ident #ty_generics>,
            }

            impl #wrapper_impl_generics #c::de::StrKeyMap for __State #wrapper_ty_generics #bounded_where_clause {
                fn key(&mut self, __k: &#c::__::str) -> #c::Result<&mut dyn #c::de::Visitor> {
                    match __k {
                        #(
                            #fieldstr => #c::__::Ok(#c::Deserialize::begin(&mut self.#fieldname)),
                        )*
                        _ => #c::__::Ok(#c::de::Visitor::ignore()),
                    }
                }

                fn finish(self: #c::__::Box<Self>) -> #c::Result<()> {
                    #(
                        let #fieldname = self.#fieldname.ok_or(#c::Error)?;
                    )*
                    *self.__out = #c::__::Some(#ident {
                        #(
                            #fieldname,
                        )*
                    });
                    #c::__::Ok(())
                }
            }
        };
    })
}

pub fn derive_enum(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let c = crate::frontend();

    let (intro_generics, fwd_generics, _) = input.generics.split_for_impl();
    let bound = parse_quote!(#c::Deserialize);
    let where_clause = bound::where_clause_with_bound(&input.generics, bound);

    let Enum = &input.ident;
    let dummy = Ident::new(
        &format!("_IMPL_DESERIALIZE_FOR_{}", Enum),
        Span::call_site(),
    );

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
            #[repr(C)]
            struct __Visitor #intro_generics
            #where_clause
            {
                __out: #c::__::Option<#Enum #fwd_generics>,
            }

            impl #intro_generics
                #c::Deserialize
            for
                #Enum #fwd_generics
            #where_clause
            {
                fn begin (__out: &'_ mut #c::__::Option<Self>)
                  -> &'_ mut dyn #c::de::Visitor
                {
                    unsafe {
                        &mut *{
                            __out
                            as *mut #c::__::Option<Self>
                            as *mut __Visitor #fwd_generics
                        }
                    }
                }
            }

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
                    self.__out = #c::__::Some(value);
                    #c::__::Ok(())
                }
            }
        )
    } else {
        todo!()
    };

    Ok(quote!(
        #[allow(non_upper_case_globals, nonstandard_style, unused_variables)]
        const #dummy: () = {
            #ret
        };
    ))
}
