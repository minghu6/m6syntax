use std::{
    collections::HashSet,
    fmt::Debug,
    iter::Peekable,
    str::FromStr,
    vec::IntoIter,
};

use crate::lexer::{
    make_token_matcher_rules, prelude::*, tokenize as tokenize_, SrcFileInfo,
    TokenMatchResult, TokenizeResult,
};

use crate::data::*;


make_token_matcher_rules! {
    lfsym     => r#"[[:alpha:]_][[:alnum:]_]*:"#,
    unpacked_sym     => r#"\.\.[[:alpha:]_][[:alnum:]_]*"#,
    nonterm   => r#"\[[[:alpha:]_][[:alnum:]_]*\]"#,
    term      => r#"<[[:alpha:]_][[:alnum:]_]*>"#,
    occ       => r#"(\+|\?|\*)"#,

    delim => "\\|",

    // Comment
    sharp_line_comment,

    // White characters
    sp,
    newline,

    // Bracket
    lparen,
    rparen,
}


trait GetName {
    fn name(&self) -> String;
}


#[derive(Debug, Default)]
pub struct BNF(pub Vec<Deriv>);


/// Derivation
#[derive(Debug)]
pub struct Deriv {
    pub lfsym: String,
    pub choices: Vec<RhStr>,
}


#[derive(Debug, Clone)]
pub struct RhStr(pub Vec<(String, Occ)>);


#[derive(Debug, Clone)]
pub enum Occ {
    /// *
    Any,
    JustOne,
    /// ?
    AtMostOne,
    /// +
    OneOrMore,
}

pub use Occ::*;

use crate::TokenTree;



#[derive(Default)]
struct Nom {
    bnf: BNF,

    lfsym: Option<String>,
    choices: Option<Vec<RhStr>>,

    rhstr: Option<RhStr>,
}





// pub struct VerifyResult<'a, T> {
//     // pub nonterm_stack: Vec<String>,
//     // pub matched_sym: Vec<String>,
//     // pub candicate_choice: Vec<RhStr>,
//     pub failed: &'a TokenTree<T>,
// }


// struct VerifySyntaxRun<'bnf, T: Debug> {
//     bnf: &'bnf BNF,
//     tt: &'bnf TokenTree<T>,

//     /// 待处理集合
//     q: VecDeque<Vec<Cow<'bnf, str>>>,
// }


// impl<'a, T: Debug> Debug for VerifyResult<'a, T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         writeln!(f,)?;

//         for (name, _) in self.failed.iter() {
//             write!(f, "{:?} ", name)?
//         }

//         writeln!(f)?;

//         Ok(())
//     }
// }


impl<T: Debug> GetName for T {
    fn name(&self) -> String {
        format!("{:?}", self)
    }
}


// impl<'bnf, T: Debug> VerifySyntaxRun<'bnf, T> {
// fn run(bnf: &'bnf BNF, tt: &'bnf TokenTree<T>) {

//     let mut it = Self {
//         bnf,
//         q: VecDeque::new(),
//         tt
//     };

//     it.run_()

// }
// }



impl Nom {
    fn new() -> Self {
        Self {
            bnf: BNF(Vec::new()),
            ..Default::default()
        }
    }


    ////////////////////////////////////////////////////////////////////////////
    //// Collect

    /// Collect choices into one deriv
    fn collect_choices(&mut self) {
        if let Some(choices) = self.choices.take() {
            self.bnf.0.push(Deriv {
                lfsym: self.lfsym.take().unwrap(),
                choices,
            })
        }
    }

    fn collect_rhstr(&mut self) {
        if let Some(rhstr) = self.rhstr.take() {
            if let Some(choices) = &mut self.choices {
                choices.push(rhstr);
            } else {
                unreachable!()
            }
        }
    }


    ////////////////////////////////////////////////////////////////////////////
    //// Open


    fn push_rhsym(&mut self, rhsym: String, occ: Occ) {
        if let Some(symstr) = &mut self.rhstr {
            symstr.0.push((rhsym, occ));
        }
    }
}


impl FromStr for Occ {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "+" => OneOrMore,
            "?" => AtMostOne,
            "*" => Any,
            _ => return Err(s.to_owned()),
        })
    }
}


impl BNF {
    fn find_lfsym(&self, lfsym: &str) -> Option<&Deriv> {
        self.0.iter().find(|x| x.lfsym == lfsym)
    }

    fn find_lfsym_mut(&mut self, lfsym: &str) -> Option<&mut Deriv> {
        self.0.iter_mut().find(|x| x.lfsym == lfsym)
    }

    /// 1. 每一个右串上的非终结符都有对应的定义
    /// 1. 没有重复定义
    pub fn self_validate(&self) -> Result<(), String> {
        let mut set = HashSet::new();

        for deriv in self.0.iter() {
            if set.contains(&deriv.lfsym) {
                return Err(format!("Duplicate define for {}", deriv.lfsym));
            }
            set.insert(&deriv.lfsym);

            for choice in deriv.choices.iter() {
                for (rhsym, _occ) in choice.0.iter() {
                    if is_nonterm(rhsym) && self.find_lfsym(&rhsym).is_none() {
                        return Err(format!("No define for [{}]", rhsym));
                    }
                }
            }
        }

        Ok(())
    }


    pub fn verify<'a, T: Debug>(
        &'a self,
        tt: &'a TokenTree<T>,
    ) -> Result<(), ()> {
        if self.0.is_empty() {
            eprintln!("No deriv found in BNF");
            return Ok(());
        }

        verify(self, tt)
    }
}


fn is_nonterm(sym: &str) -> bool {
    is_shadow_lfsym(sym)
        || (sym.chars().next().unwrap() as char).is_uppercase()
}

fn is_explicit_nonterm(sym: &str) -> bool {
    (sym.chars().next().unwrap() as char).is_uppercase()
}

fn is_shadow_lfsym(sym: &str) -> bool {
    sym.starts_with("!__")
}


fn tokenize(src: &str) -> TokenizeResult {
    let source = SrcFileInfo::from_str(src.to_owned());

    trim(tokenize_(&source, &MATCHERS[..])).and_then(|toks| {
        Ok(toks
            .into_iter()
            .map(|tok| {
                let name = tok.name_string();
                let val = tok.value_string();

                match name.as_str() {
                    "lfsym" => tok.mapval(&val.as_str()[..val.len() - 1]),
                    "nonterm" | "term" => {
                        tok.mapval(&val.as_str()[1..val.len() - 1])
                    }
                    "unpacked_sym" => tok.mapval(&val.as_str()[2..val.len()]),
                    _ => tok,
                }
            })
            .collect())
    })
}


pub type CounterType = Box<dyn FnMut() -> usize>;

fn gen_counter() -> CounterType {
    Box::new(gen_counter_1(1))
}

fn gen_counter_1(init: usize) -> CounterType {
    let mut count = init;

    Box::new(move || {
        let old_count = count;
        count += 1;
        old_count
    })
}


fn shadow_lfsym(paren: &str, count: usize) -> String {
    format!("!__{}_{}", paren, count)
}


fn parse(src: &str) -> BNF {
    let mut nom = Nom::new();

    let mut tokens = tokenize(src).unwrap().into_iter().peekable();
    let mut auto = gen_counter();

    while let Some(tok) = tokens.next() {
        let name = tok.name_string();
        let value = tok.value_string();

        match name.as_str() {
            "lfsym" => {
                nom.collect_rhstr();
                nom.collect_choices();

                nom.lfsym = Some(value);
                nom.choices = Some(Vec::new());
                auto = gen_counter();
            }
            // 把一个既有的 nonterm choices 全部加入到当前解析的 lfsym 的 choices 中去。
            // 只是一个简单的语法糖，方便定义既有 nonterm。的别名或者超集。
            "unpacked_sym" => {
                if let Some(deriv) = nom.bnf.find_lfsym_mut(&value) {
                    if let Some(choices) = &mut nom.choices {
                        choices.extend(deriv.choices.iter().cloned());
                    }
                } else {
                    unreachable!("Unkonwn unpacked symbol: {value}")
                }
            }
            // start choices
            "delim" => {
                nom.collect_rhstr();

                nom.rhstr = Some(RhStr(Vec::new()));
            }
            "nonterm" | "term" => {
                let rhsym = value;
                let occ = parse_occ(&mut tokens);

                nom.push_rhsym(rhsym, occ);
            }
            "lparen" => {
                let shadow_sym =
                    shadow_lfsym(&nom.lfsym.clone().unwrap(), auto());
                nom.bnf.0.extend(parse_grouped_deriv(
                    &mut tokens,
                    shadow_sym.clone(),
                ));
                let occ = parse_occ(&mut tokens);

                nom.push_rhsym(shadow_sym, occ)
            }
            _ => unreachable!("tok: {tok:?}"),
        };

        // println!("{tok:?}")
    }

    // tail recycle
    nom.collect_rhstr();
    nom.collect_choices();

    nom.bnf
}


/// 语法上允许嵌套
fn parse_grouped_deriv(
    tokens: &mut Peekable<IntoIter<Token>>,
    lfsym: String,
) -> Vec<Deriv> {
    let mut derives = vec![];
    let mut choices = vec![];
    let mut rhstr = vec![];
    let mut auto = gen_counter();

    while let Some(tok) = tokens.next() {
        let name = tok.name_string();
        let value = tok.value_string();

        match name.as_str() {
            "nonterm" | "term" => {
                let rhsym = value;
                let occ = parse_occ(tokens);

                rhstr.push((rhsym, occ));
            }
            "lparen" => {
                // recursively invoke
                let shadow_sym = shadow_lfsym(&lfsym, auto());
                derives
                    .extend(parse_grouped_deriv(tokens, shadow_sym.clone()));
                let occ = parse_occ(tokens);
                rhstr.push((shadow_sym, occ));
            }
            "rparen" => {
                choices.push(RhStr(rhstr));
                break;
            }
            "delim" => {
                choices.push(RhStr(rhstr));
                rhstr = vec![];
            }
            _ => unreachable!("tok: {tok:?}"),
        }
    }

    derives.push(Deriv { lfsym, choices });

    derives
}


fn parse_occ(tokens: &mut Peekable<IntoIter<Token>>) -> Occ {
    if let Some(tok1) = tokens.peek() && tok1.check_name("occ") {
        let tok_occ = tokens.next().unwrap();
        tok_occ.value_string().parse().unwrap()
    }
    else {
        JustOne
    }
}


pub fn gen_bnf(src: &str) -> BNF {
    let bnf = parse(src);

    bnf.self_validate().unwrap();

    bnf
}


fn verify<'a, T: Debug>(
    bnf: &'a BNF,
    tt: &'a TokenTree<T>,
) -> Result<(), ()> {
    let ent = &bnf.0[0];
    let top_tt = tt;

    // let mut path = vec![ent];
    let mut q: Vec<(&Deriv, &TokenTree<T>)> = vec![(ent, top_tt)];

    /* Top down BFS */
    while !q.is_empty() {
        let mut nxt_q = vec![];

        for (deriv, tt) in q.into_iter() {
            let p1 = Cursor::new(tt.len());

            let mut failed = false;

            match verify_rhstr(bnf, deriv, tt, p1) {
                Ok((p1, chidren)) => {
                    if p1.reach_end() {
                        nxt_q.extend(chidren);
                    }
                    else {
                        eprint!("{} REM: ", deriv.lfsym);
                        for (sym, _) in tt[*p1..].iter() {
                            eprint!("{}, ", sym.name())
                        }
                        eprintln!();

                        failed = true;
                    }
                },
                Err(failed_at) => {
                    for (choice, actual_sym) in deriv.choices.iter().zip(failed_at) {
                        eprintln!("{} :- {:?}", deriv.lfsym, choice.0);
                        eprintln!("failed at {}", actual_sym);
                    }

                    failed = true;
                },
            }

            if failed {
                eprintln!();
                eprint!("Actual: ");
                for (name, _) in tt.iter() {
                    print!("{name:?} ");
                }
                eprintln!();

                return Err(());
            }
        }

        q = nxt_q;
    }

    Ok(())
}


fn verify_rhstr<'v, T: Debug>(
    bnf: &'v BNF,
    deriv: &'v Deriv,
    tt: &'v TokenTree<T>,
    p1: Cursor,
) -> Result<(Cursor, Vec<(&'v Deriv, &'v TokenTree<T>)>), Vec<String>> {
    let mut ok_choices = vec![];
    let mut failed_at = vec![];
    let backup_p1 = p1;

    for choice in deriv.choices.iter() {
        let mut p0 = Cursor::new(choice.0.len());
        let mut p1 = backup_p1;
        let mut repeat = 0;
        let mut failed = false;

        let mut children = vec![];

        while !p1.reach_end() && !p0.reach_end() {
            let (formal_sym, occ) = &choice.0[*p0];
            let actual_sym = tt[*p1].0.name();

            let mut matched = false;
            /* expand shadow lfsym */
            if is_shadow_lfsym(&formal_sym) {
                let shadow_deriv = bnf.find_lfsym(&formal_sym).unwrap();

                if let Ok((_p1, part_children)) =
                    verify_rhstr(bnf, shadow_deriv, tt, p1)
                {
                    children.extend(part_children);

                    repeat += 1;
                    p1 = _p1;
                    matched = true;
                }
            } else if *formal_sym == actual_sym {
                if is_explicit_nonterm(&formal_sym) {
                    children.push((
                        bnf.find_lfsym(&formal_sym).unwrap(),
                        tt[*p1].1.as_tt(),
                    ))
                }

                repeat += 1;
                p1.inc();

                matched = true;
            }

            if matched {
                if matches!(occ, JustOne | AtMostOne) {
                    p0.inc();
                    repeat = 0;
                }

                continue;
            }

            if matches!(occ, Any | AtMostOne)
                || matches!(occ, OneOrMore) && repeat > 0
            {
                p0.inc();
                repeat = 0;

                continue;
            }

            /* Failed */
            failed = true;
            failed_at.push(actual_sym);

            break;
        }

        if !failed {
            ok_choices.push((choice, p1, children));
        }
    }


    if ok_choices.is_empty() {
        Err(failed_at)
    } else if ok_choices.len() == 1 {
        Ok({
            let (_, cursor, children) = ok_choices.pop().unwrap();

            (cursor, children)
        })
    } else {
        // unreachable!(
        //     "Bad BNF, duplicate choices found: {:#?}",
        //     ok_choices.into_iter().map(|x| x.0).collect::<Vec<&RhStr>>()
        // )

        let (_, cursor, children) = ok_choices
            .into_iter()
            .next()
            .unwrap();

        Ok((cursor, children))
    }
}
