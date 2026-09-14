use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, Token, Visibility,
    token::{ Bracket, Lt }, bracketed
};



struct GenSyntaxEnum {
    visib: Visibility,
    enum_name: Ident,
    names: Punctuated<Ident, Token![,]>,
}



struct BNF(Vec<Deriv>);


/// Derivation
struct Deriv {
    pub lfsym: Ident,
    pub choices: Vec<RhStr>,
}


struct RhStr(Vec<(RhSym, Occ)>);


enum Occ {
    /// *
    Any,

    JustOne,

    /// ?
    AtMostOne,

    /// +
    OneOrMore,
}


struct RhSym(String);


////////////////////////////////////////////////////////////////////////////////
//// Parse

// macro_rules! peek_parse {
//     ($input:ident, $toke:expr, $tokt:ty, $name:literal) => {
//         if $input.peek($toke) {
//             $input.parse::<$tokt>()?;
//             return Ok(Self($name.to_string()))
//         }
//     };
// }


impl Parse for RhSym {
    fn parse(input: ParseStream) -> Result<Self> {

        // peek_parse!(input, Token![,], Token![,], "comma");

        Ok(Self(input.parse::<Ident>()?.to_string()))
    }
}


impl Parse for Occ {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(
            if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;

                Self::Any
            }
            else if input.peek(Token![?]) {
                input.parse::<Token![?]>()?;

                Self::AtMostOne
            }
            else if input.peek(Token![+]) {
                input.parse::<Token![+]>()?;

                Self::OneOrMore
            }
            else {
                Self::JustOne
            }
        )
    }
}


impl Parse for RhStr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rhstr = vec![];

        while input.peek(Bracket) || input.peek(Lt) {
            let rhsym;

            if input.peek(Bracket) {
                let rhsym_content;
                bracketed!(rhsym_content in input);
                rhsym = rhsym_content.parse::<RhSym>()?;
            }
            else {
                input.parse::<Token![<]>()?;
                rhsym = input.parse::<RhSym>()?;
                input.parse::<Token![>]>()?;
            }

            let repeat = input.parse::<Occ>()?;

            rhstr.push((rhsym, repeat));
        }

        Ok(Self(rhstr))

    }
}


impl Parse for Deriv {
    fn parse(input: ParseStream) -> Result<Self> {
        let lfsym = input.parse::<Ident>()?;
        let mut choices = vec![];
        input.parse::<Token![:]>()?;

        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;

            let rhstr = input.parse::<RhStr>()?;
            choices.push(rhstr);
        }

        Ok(Self { lfsym, choices })
    }
}


impl Parse for BNF {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut derivs = vec![];

        while !input.is_empty() {
            derivs.push(input.parse::<Deriv>()?);
        }

        Ok(Self(derivs))
    }
}



impl Parse for GenSyntaxEnum {
    fn parse(input: ParseStream) -> Result<Self> {
        let visib = input.parse::<Visibility>()?;
        let enum_name = input.parse::<Ident>()?;
        input.parse::<Token![|]>()?;
        let names = input.parse_terminated(Ident::parse)?;

        Ok(Self {
            visib,
            enum_name,
            names,
        })
    }
}




////////////////////////////////////////////////////////////////////////////////
//// Generate

#[proc_macro]
pub fn grammer(input: TokenStream) -> TokenStream {

    let BNF(derives) = parse_macro_input!(input as BNF);

    let mut derive_push = quote! {
        let mut bnf = BNF::new();
    };

    for deriv in derives {
        let Deriv {lfsym, choices} = deriv;

        let mut choice_push = quote! {
            let mut choices = vec![];
        };

        for rhstr in choices {
            let mut rhstr_push = quote!{ let mut rhstr = vec![]; };

            for (rhsym, repeat) in rhstr.0 {
                let occ =
                match repeat {
                    Occ::Any => quote!(Any),
                    Occ::JustOne => quote!(JustOne),
                    Occ::AtMostOne => quote!(AtMostOne),
                    Occ::OneOrMore => quote!(OneOrMore),
                };

                let rhsymstr = rhsym.0;
                let gen_rhsym = quote! {
                    format!("{}", #rhsymstr)
                };

                rhstr_push.extend(
                    quote! {
                        rhstr.push((
                            #gen_rhsym,
                            #occ
                        ));
                    }
                );
            }

            let rhstr_gen = quote!{
                {
                    #rhstr_push
                    rhstr
                }
            };

            choice_push.extend(
                quote! {
                    choices.push(RhStr(#rhstr_gen));
                }
            );
        }

        let choice_gen = quote! {
            {
                #choice_push
                choices
            }
        };

        derive_push.extend(
            quote! {
                bnf.0.push(Deriv{ lfsym: stringify!(#lfsym).to_string(), choices: #choice_gen });
            }
        )
    }

    TokenStream::from(quote! {
        {
            #derive_push
            bnf
        }
    })
}



#[proc_macro]
pub fn gen_syntax_enum(input: TokenStream) -> TokenStream {
    let GenSyntaxEnum {
        visib,
        enum_name,
        names,
    } = parse_macro_input!(input as GenSyntaxEnum);

    let mut enum_units_ts = quote! {};
    let mut match_display_ts = quote! {};
    let mut match_name_ts = quote! {};

    for name in names {
        enum_units_ts.extend(quote! { #name, });

        let name_s = name.to_string();
        let name_s = name_s.trim_start_matches("r#");

        if name_s.chars().next().unwrap().is_uppercase() {
            // NonTerminal
            match_display_ts.extend(quote! {
                Self::#name => write!(f, "[{}]", stringify!(#name)),
            })
        } else {
            match_display_ts.extend(quote! {
                Self::#name => write!(f, "<{}>", stringify!(#name)),
            })
        }

        match_name_ts.extend(quote! {
            Self::#name => #name_s,
        });
    }

    TokenStream::from(quote! {
        #[allow(unused)]
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #visib enum #enum_name {
            #enum_units_ts
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #match_display_ts
                }
            }
        }

        impl #enum_name {
            #visib fn name(&self) -> String {
                match self {
                    #match_name_ts
                }.to_owned()
            }
        }
    })
}


