use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, parse_quote};

/// Protects users from low-level traits by donning a [hazmat suit].
///
/// A frequent desire in cryptographic crates is to provide safe high-level functionality
/// that relies on a hazardous low-level primitive, while enabling that primitive to be
/// provided by external backends.
///
/// For example, the [`rsa`] crate provides implementations of various RSA schemes, that
/// all build on the core "textbook RSA" encryption primitive. It is [very unsafe] to use
/// this primitive directly! However, hardware tokens that use the [PIV card interface]
/// only expose this primitive, so to allow the RSA scheme implementations to be reused by
/// arbitrary hardware tokens, the primitive's interface needs to be exposed via a trait.
///
/// This procedural macro enables crate authors to easily split their traits into "public"
/// traits (providing APIs for users), and "implement-only" traits (providing APIs for
/// backend developers). It does this by augmenting the implement-only traits' methods
/// with a [capability]: an extra argument that must be provided when the method is
/// called. The capability's type is generated by the procedural macro such that only the
/// trait author is able to create capability instances. This allows anyone to implement
/// the trait (because the capability argument can be ignored), but usage of the trait can
/// be tightly controlled by the crate in which the trait is defined.
///
/// [hazmat suit]: https://en.wikipedia.org/wiki/Hazmat_suit
/// [`rsa`]: https://crates.io/crates/rsa
/// [very unsafe]: https://link.springer.com/content/pdf/10.1007/3-540-44448-3_3.pdf
/// [PIV card interface]: https://pages.nist.gov/FIPS201/system/
/// [capability]: https://en.wikipedia.org/wiki/Capability-based_security
///
/// # Examples
///
/// To make a trait implement-only, add the `#[hazmat::suit]` attribute:
///
/// ```
/// # mod hazmat {
/// #    pub use hazmat_macros::suit;
/// # }
/// /// This is a low-level trait that we want downstream users to implement but not use.
/// /// We protect it by putting on a [`hazmat::suit`].
/// #[hazmat::suit]
/// pub trait AddOnce {
///     fn add_once(self, other: &Self) -> Self;
/// }
///
/// /// This is a high-level trait that we want downstream users to use.
/// pub trait AddTwice {
///     fn add_twice(self, other: &Self) -> Self;
/// }
///
/// // We provide the high-level implementation in terms of the low-level implementation,
/// // allowing downstream users to customise the internals without exposing the internals
/// // in the public API.
/// //
/// // More precisely, the low-level implementation _is_ part of the public API, but it
/// // cannot be used publicly because the capability type can only be constructed by the
/// // trait author.
/// impl<T: AddOnce> AddTwice for T {
///     fn add_twice(self, other: &Self) -> Self {
///         self.add_once(other, AddOnceCap).add_once(other, AddOnceCap)
///     }
/// }
/// ```
///
/// This trait can then be implemented directly by downstream crate users:
///
/// ```
/// # mod upstream_crate {
/// #     #[hazmat_macros::suit]
/// #     pub trait AddOnce {
/// #         fn add_once(self, other: &Self) -> Self;
/// #     }
/// # }
/// struct MyNum(u32);
///
/// impl upstream_crate::AddOnce for MyNum {
///     fn add_once(self, other: &Self, _cap: upstream_crate::AddOnceCap) -> Self {
///         Self(self.0 + other.0)
///     }
/// }
/// ```
///
/// The downstream crate users can also add the `#[hazmat::suit]` attribute to their trait
/// implementation to simplify it:
///
/// ```
/// # mod hazmat {
/// #    pub use hazmat_macros::suit;
/// # }
/// # mod upstream_crate {
/// #     #[hazmat_macros::suit]
/// #     pub trait AddOnce {
/// #         fn add_once(self, other: &Self) -> Self;
/// #     }
/// # }
/// struct MyNum(u32);
///
/// #[hazmat::suit]
/// impl upstream_crate::AddOnce for MyNum {
///     fn add_once(self, other: &Self) -> Self {
///         Self(self.0 + other.0)
///     }
/// }
/// ```
///
/// Note that if the upstream trait is imported directly, then `#[hazmat::suit]` applied
/// to the trait implementation will require that the capability is also imported:
///
/// ```
/// # mod hazmat {
/// #    pub use hazmat_macros::suit;
/// # }
/// # mod upstream_crate {
/// #     #[hazmat_macros::suit]
/// #     pub trait AddOnce {
/// #         fn add_once(self, other: &Self) -> Self;
/// #     }
/// # }
/// use upstream_crate::{AddOnce, AddOnceCap};
///
/// struct MyNum(u32);
///
/// #[hazmat::suit]
/// impl AddOnce for MyNum {
///     fn add_once(self, other: &Self) -> Self {
///         Self(self.0 + other.0)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn suit(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Parse the `TokenStream` into a syntax tree.
    let item = parse_macro_input!(item as syn::Item);

    let augmented = match item {
        // When applied to a trait, restrict the trait's methods with a capability.
        syn::Item::Trait(t) => augment_trait(t),

        // When applied to a trait impl, append the expected capability argument.
        syn::Item::Impl(t) if t.trait_.is_some() => augment_trait_impl(t),

        // When applied to any other kind of item, generate a compiler error.
        _ => syn::Error::new_spanned(
            item,
            "hazmat::suit should be applied to traits or trait impls",
        )
        .into_compile_error(),
    };
    augmented.into()
}

fn augment_trait(mut t: syn::ItemTrait) -> TokenStream {
    // Create a name for the capability corresponding to this trait.
    let cap_name = syn::Ident::new(&format!("{}Cap", t.ident), Span::call_site());

    // Modify the trait to add the capability to each method as an argument.
    for item in &mut t.items {
        if let syn::TraitItem::Method(method) = item {
            let cap_arg = syn::PatType {
                attrs: vec![],
                pat: parse_quote!(cap),
                colon_token: parse_quote!(:),
                ty: parse_quote!(#cap_name),
            };
            method.sig.inputs.push(cap_arg.into());
        }
    }

    quote! {
        #[non_exhaustive]
        pub struct #cap_name;

        #t
    }
}

fn augment_trait_impl(mut t: syn::ItemImpl) -> TokenStream {
    // Create a name for the capability corresponding to this trait.
    let trait_path = &t.trait_.as_ref().unwrap().1;
    let trait_name = &trait_path.segments.last().unwrap().ident;
    let cap_name = syn::Ident::new(&format!("{}Cap", trait_name), Span::call_site());
    let cap_path = {
        let mut p = trait_path.clone();
        p.segments.pop();
        p.segments.push(cap_name.into());
        p
    };

    // Modify the trait implementation to add the capability to each method.
    for item in &mut t.items {
        if let syn::ImplItem::Method(method) = item {
            let cap_arg = syn::PatType {
                attrs: vec![],
                pat: parse_quote!(cap),
                colon_token: parse_quote!(:),
                ty: parse_quote!(#cap_path),
            };
            method.sig.inputs.push(cap_arg.into());
        }
    }

    quote! {
        #t
    }
}
