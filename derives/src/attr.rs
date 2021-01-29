use ::core::ops::Not as _;
use ::syn::{spanned::Spanned, Result, *};

/// Find the value of a #[serde(rename = "...")] attribute.
fn attr_rename(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut ret = None;

    for_each_serde_attr!( attrs =>
        #[serde( rename = $new_name )] => {
            let prev = ret.replace(new_name);
            if prev.is_some() {
                return Err(Error::new_spanned(rename, "duplicate `rename` attribute"));
            }
        },

        #[serde( with = "serde_bytes" )] => {
            // Thanks to `view_seq` and the impl for `u8`, we have already specialized
            // the "sequence of u8s" case, so no need for `serde_bytes`.
            // Thus, nothing to do.
        },

        #[serde(other)] => {
            // This is sometimes correct; ignore it since it will be correctly
            // handled somewhere else. FIXME: do this better.
        },

        #[serde(skip)] => {

        },
    )?;

    Ok(ret)
}

pub fn has_skip_deserializing(attrs: &[Attribute]) -> bool {
    let mut ret = false;
    let _ = for_each_serde_attr! { attrs =>
        #[serde(skip_deserializing)] => ret = true,
        #[serde(skip)] => ret = true,
        _ => {},
    };
    ret
}

// pub fn has_skip_serializing(attrs: &[Attribute]) -> bool {
//     let mut ret = false;
//     let _ = for_each_serde_attr! { attrs =>
//         #[serde(skip_serializing)] => ret = true,
//         #[serde(skip)] => ret = true,
//         _ => {},
//     };
//     ret
// }

/// Determine the name of a field, respecting a rename attribute.
pub fn name_of_field(field: &Field) -> Result<String> {
    let rename = attr_rename(&field.attrs)?;
    Ok(rename.unwrap_or_else(|| field.ident.as_ref().unwrap().to_string()))
}

/// Determine the name of a variant, respecting a rename attribute.
pub fn name_of_variant(var: &Variant) -> Result<String> {
    let rename = attr_rename(&var.attrs)?;
    Ok(rename.unwrap_or_else(|| var.ident.to_string()))
}

#[derive(Debug)] // FIXME: remove this.
pub enum EnumTaggingMode {
    ExternallyTagged,
    InternallyTagged {
        tag_name: String,
        content_name: Option<String>,
    },
    Untagged,
}

#[rustfmt::skip]
impl EnumTaggingMode {
    pub
    fn from_attrs (attrs: &'_ [Attribute])
      -> Result<EnumTaggingMode>
    {
        let mut ret = None;
        let mut last_content = None;

        for_each_serde_attr!( attrs =>
            #[serde( tag = $tag_name )] => {
                let prev = ret.replace(EnumTaggingMode::InternallyTagged {
                    tag_name,
                    content_name: last_content.take().map(|(it, _)| it),
                });

                if prev.is_some() {
                    return Err(Error::new_spanned(tag, "duplicate `tag` attribute"));
                }
            },

            #[serde( content = $content_name )] => match ret {
                None => if last_content.replace((content_name, content.span())).is_some() {
                    return Err(Error::new_spanned(content, "duplicate `content` attribute"));
                },
                Some(EnumTaggingMode::InternallyTagged {
                    content_name: ref mut out_content_name @ None,
                    ..
                }) => {
                    *out_content_name = Some(content_name);
                },
                Some(_) => {
                    return Err(Error::new_spanned(content, "Extraneous `content` attribute"));
                },
            },

            #[serde( untagged )] => {
                let prev = ret.replace(EnumTaggingMode::Untagged);
                if prev.is_some() {
                    return Err(Error::new_spanned(
                        untagged,
                        "Contradicts a previously-encountered `tag` attribute",
                    ));
                }
            },
        )?;

        if let Some((_, span)) = last_content {
            Err(Error::new(span, "Extraneous `content` attribute"))
        } else {
            Ok(ret.unwrap_or_else(|| EnumTaggingMode::ExternallyTagged))
        }
    }
}

#[cfg_attr(rustfmt, rustfmt::skip)]
macro_rules! for_each_serde_attr {
    (
        @[acc = $($acc:tt)*]
        #[serde(
            $key:ident = $__:tt $value:ident
        )] => $body:expr $(,
        $($rest:tt)* )?
    ) => (for_each_serde_attr! {
        @[acc = $($acc)*
            match meta!() {
                | Meta::NameValue(MetaNameValue {
                    path,
                    lit: Lit::Str(s),
                    ..
                })
                    if path.is_ident(stringify!($key))
                => {
                    let $key = path;
                    let _ = $key;
                    let $value = s.value();
                    return Some((|| Ok::<(), ::syn::Error>({
                        $body
                    }))());
                },
                | _ => {},
            }
        ]
        $($($rest)*)?
    });

    (
        @[acc = $($acc:tt)*]
        #[serde(
            $key:ident = $str_lit:literal
        )] => $body:expr $(,
        $($rest:tt)* )?
    ) => (for_each_serde_attr! {
        @[acc = $($acc)*
            match meta!() {
                | Meta::NameValue(MetaNameValue {
                    path,
                    lit: Lit::Str(s),
                    ..
                })
                    if path.is_ident(stringify!($key))
                    && s.value() == $str_lit
                => {
                    return Some((|| Ok::<(), ::syn::Error>({
                        $body
                    }))());
                },

                | _ => {},
            }
        ]
        $($($rest)*)?
    });

    (
        @[acc = $($acc:tt)*]
        #[serde(
            $key:ident
        )] => $body:expr $(,
        $($rest:tt)* )?
    ) => (for_each_serde_attr! {
        @[acc = $($acc)*
            match meta!() {
                | Meta::Path(path) if path.is_ident(stringify!($key)) => {
                    let $key = path;
                    let _ = $key;
                    return Some((|| Ok::<(), ::syn::Error>(
                        $body
                    ))());
                },
                | _ => {},
            }
        ]
        $($($rest)*)?
    });

    (
        @[acc = $($acc:tt)*]
        _ $(if $guard:expr)? => $last_branch:expr $(,
        $($rest:tt)* )?
    ) => (for_each_serde_attr! {
        @[acc = $($acc)*
            if true $(&& $guard)? {
                return Some((|| Ok::<(), ::syn::Error>( $last_branch ))());
            }
        ]
        $($($rest)*)?
    });

    (
        @[acc = $($acc:tt)*]
        /* Nothing left: default branch -> error */
    ) => ({
        $($acc)*

        None
    });

    (
        $attrs:expr =>
        $($input:tt)*
    ) => (
        try_for_each_serde_attr($attrs, |meta| {
            macro_rules! meta {() => ( meta )}
            for_each_serde_attr! {
                @[acc = ]
                $($input)*
            }
        })
    );
}
use for_each_serde_attr;

#[rustfmt::skip]
fn try_for_each_serde_attr (
    attrs: &'_ [Attribute],
    mut f: impl FnMut(&'_ Meta) -> Option<Result<()>>,
) -> Result<()>
{
    for attr in attrs {
        if attr.path.is_ident("serde").not() {
            continue;
        }
        let list = match attr.parse_meta()? {
            | Meta::List(list) => list,
            | other => return Err(Error::new_spanned(other, "invalid attribute")),
        };
        for meta in &list.nested {
            if let NestedMeta::Meta(ref meta) = *meta {
                match f(meta) {
                    | Some(Ok(())) => continue,
                    | Some(err) => return err,
                    | None => {}
                }
            }
            return Err(Error::new_spanned(meta, "invalid attribute"));
        }
    }
    Ok(())
}
