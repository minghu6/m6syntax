extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
};


////////////////////////////////////////////////////////////////////////////////
//// Utils

struct CratePathPrefix {
    crate_path: syn::Path,
    rest: TokenStream,
}

impl Parse for CratePathPrefix {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let crate_path = input.parse()?;
        input.parse::<Token![,]>()?;
        let rest = input.parse::<proc_macro2::TokenStream>()?.into();
        Ok(Self { crate_path, rest })
    }
}

////////////////////////////////////////////////////////////////////////////////
//// MakeCharMatcherRules

struct MakeCharMatcherRules {
    // ident, patstr, matcher_t
    rules: Vec<(Ident, LitStr, Ident)>,
}

impl Parse for MakeCharMatcherRules {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rules = vec![];

        while !input.is_empty() {
            let name = input.parse()?;
            input.parse::<Token!(=>)>()?;
            let patstr = input.parse()?;
            input.parse::<Token!(|)>()?;
            let matcher_t = input.parse()?;

            if !input.is_empty() {
                input.parse::<Token!(,)>()?;
            }

            rules.push((name, patstr, matcher_t));
        }

        Ok(Self { rules })
    }
}

#[doc(hidden)]
#[proc_macro]
pub fn __make_char_matcher_rules(input: TokenStream) -> TokenStream {
    let CratePathPrefix { crate_path, rest } = parse_macro_input!(input);

    let MakeCharMatcherRules { rules } =
        parse_macro_input!(rest as MakeCharMatcherRules);

    let mut token_stream = quote! {
        use #crate_path::{
            Token,
            SrcLoc,
            Symbol,
            RegexCharMatcher,
            SimpleCharMatcher,
            CharMatcher,
            lazy_static::lazy_static
        };
    };

    for (name, patstr, matcher_t) in rules {
        let matcher_fn_name = Ident::new(
            &format!("{}_m", name.to_string().to_lowercase()),
            Span::call_site(),
        );
        let matcher_reg_name = Ident::new(
            &format!("{}_REG", name.to_string().to_uppercase()),
            Span::call_site(),
        );

        if matcher_t.to_string() == "r" {
            token_stream.extend(quote! {
                #[inline]
                pub fn #matcher_fn_name(c: char) -> bool {
                    lazy_static! {
                        static ref #matcher_reg_name: Box<dyn CharMatcher + Send + Sync>
                        = Box::new(RegexCharMatcher::new(#patstr));
                    }

                    #matcher_reg_name.is_match(c)
                }
            })
        } else {
            token_stream.extend(quote! {
                #[inline]
                pub fn #matcher_fn_name(c: char) -> bool {
                    lazy_static! {
                        static ref #matcher_reg_name: Box<dyn CharMatcher + Send + Sync>
                        = Box::new(SimpleCharMatcher::new(#patstr));
                    }

                    #matcher_reg_name.is_match(c)
                }
            })
        }
    }

    TokenStream::from(token_stream)
}


////////////////////////////////////////////////////////////////////////////////
//// TokenMatcher

#[allow(unused)]
struct TokenMatcherRules {
    // ident, patstr
    rules: Vec<(Ident, Option<LitStr>)>,
}

impl Parse for TokenMatcherRules {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rules = vec![];

        while !input.is_empty() {
            let name = input.parse()?;

            if input.peek(Token![=>]) {
                input.parse::<Token![=>]>()?;
                let patstr = input.parse::<LitStr>()?;
                rules.push((name, Some(patstr)))
            } else {
                rules.push((name, None))
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { rules })
    }
}

#[doc(hidden)]
#[proc_macro]
pub fn __make_token_matcher_rules(input: TokenStream) -> TokenStream {
    let CratePathPrefix { crate_path, rest } = parse_macro_input!(input);

    let TokenMatcherRules { rules } =
        parse_macro_input!(rest as TokenMatcherRules);

    let mut token_stream = quote! {
        use #crate_path::{
            Token,
            SrcLoc
        };
    };

    let mut matchers_ts = quote! {};

    for (name, patstr_opt) in rules {
        let matcher_fn_name = Ident::new(
            &format!("{}_m", name.to_string().to_lowercase()),
            Span::call_site(),
        );

        if let Some(patstr) = patstr_opt {
            let matcher_reg_name = Ident::new(
                &format!("{}_REG", name.to_string().to_uppercase()),
                Span::call_site(),
            );
            let adjust_patstr = LitStr::new(
                &format!("^({})", patstr.value()),
                Span::call_site(),
            );

            token_stream.extend(quote! {
                pub fn #matcher_fn_name(s: &str, from: usize) -> Option<TokenMatchResult> {
                    #crate_path::lazy_static::lazy_static! {
                        static ref #matcher_reg_name: #crate_path::TokenMatcher
                            = #crate_path::TokenMatcher::new(#adjust_patstr, stringify!(#name));
                    }

                    #matcher_reg_name.fetch_tok(s, from)
                }
            });
        }

        matchers_ts
            .extend(quote! { #matcher_fn_name as #crate_path::FnMatcher, });
    }

    token_stream.extend(quote! {
        #crate_path::lazy_static::lazy_static! {
            pub static ref MATCHERS: Vec<#crate_path::FnMatcher> = vec![#matchers_ts];
        }
    });

    TokenStream::from(token_stream)
}
