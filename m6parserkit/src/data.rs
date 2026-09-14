use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
    slice::SliceIndex,
};

use crate::lexer::{Span, Token};


pub struct TokenTree<T> {
    pub subs: Vec<(T, SyntaxNode<T>)>,
}


#[derive(Debug)]
pub enum SyntaxNode<T> {
    T(TokenTree<T>),
    E(Token),
}


#[derive(Clone, Copy)]
pub struct Cursor {
    p: usize,
    end: usize,
}



impl Cursor {
    pub fn new(len: usize) -> Self {
        Self { p: 0, end: len }
    }

    pub fn inc(&mut self) {
        self.p += 1;
    }

    #[allow(unused)]
    pub fn get(&self) -> Option<usize> {
        if !self.reach_end() {
            Some(self.p)
        } else {
            None
        }
    }

    pub fn reach_end(&self) -> bool {
        self.p >= self.end
    }
}


impl std::ops::Deref for Cursor {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.p
    }
}


impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.p)
    }
}


impl<T: Debug> SyntaxNode<T> {
    pub fn as_tt(&self) -> &TokenTree<T> {
        match self {
            Self::T(tt) => tt,
            Self::E(_) => unreachable!("{:?}", self),
        }
    }

    pub fn into_tt(self) -> TokenTree<T> {
        match self {
            Self::T(tt) => tt,
            Self::E(_) => unreachable!("{:?}", self),
        }
    }

    pub fn as_tok(&self) -> &Token {
        match self {
            Self::T(_) => unreachable!("{:?}", self),
            Self::E(tok) => tok,
        }
    }

    pub fn tok_1st(&self) -> Option<&Token> {
        match self {
            Self::T(tt) => tt.tok_1st(),
            Self::E(tok) => Some(tok),
        }
    }

    pub fn tok_last(&self) -> Option<&Token> {
        match self {
            Self::T(tt) => tt.tok_last(),
            Self::E(tok) => Some(tok),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::T(tt) => tt.span(),
            Self::E(tok) => tok.span,
        }
    }


}


impl<T: Debug> TokenTree<T> {
    pub fn new(subs: Vec<(T, SyntaxNode<T>)>) -> Self {
        Self { subs }
    }

    pub fn len(&self) -> usize {
        self.subs.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(T, SyntaxNode<T>)> {
        self.subs.iter()
    }

    pub fn tok_1st(&self) -> Option<&Token> {
        for (_, sn) in self.subs.iter() {
            if let Some(ref tok) = sn.tok_1st() {
                return Some(tok);
            }
        }

        None
    }

    pub fn tok_last(&self) -> Option<&Token> {
        for (_, sn) in self.subs.iter().rev() {
            if let Some(ref tok) = sn.tok_last() {
                return Some(tok);
            }
        }

        None
    }

    pub fn span(&self) -> Span {
        if let Some(tok_1st) = self.tok_1st() &&
           let Some(tok_last) = self.tok_last()
        {
            Span {
                from: tok_1st.span.from,
                end: tok_last.span.end,
            }
        }
        else {
            Span::default()
        }
    }

    pub fn move_elem(&mut self, i: usize) -> (T, SyntaxNode<T>) where T: Default {
        std::mem::replace(&mut self.subs[i], (T::default(), SyntaxNode::E(Token::eof())))
    }
}


impl<T: Debug> std::fmt::Debug for TokenTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbs = &mut f.debug_tuple("=>");

        for (ty, sn) in self.subs.iter() {
            dbs = dbs.field(&ty);
            match &*sn {
                SyntaxNode::T(tt) => dbs = dbs.field(&tt),
                SyntaxNode::E(tok) => dbs = dbs.field(&format!("<{:?}>", tok.value)),
            }
        }

        dbs.finish()
    }
}


impl<T, I: SliceIndex<[(T, SyntaxNode<T>)]>> Index<I> for TokenTree<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&self.subs, index)
    }
}


impl<T, I: SliceIndex<[(T, SyntaxNode<T>)]>> IndexMut<I> for TokenTree<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut self.subs, index)
    }
}

