pub use data::*;
pub use infix_expr::*;
pub use proc_macros::*;
pub use spec::*;

mod data;
mod infix_expr;
mod spec;


#[macro_export]
macro_rules! make_char_matcher_rules {
    ($($tt:tt)*) => {
        $crate::lexer::__make_char_matcher_rules!($crate::lexer, $($tt)*);
    };
}

#[macro_export]
macro_rules! make_token_matcher_rules {
    ($($tt:tt)*) => {
        $crate::lexer::__make_token_matcher_rules!($crate::lexer, $($tt)*);
    };
}


pub mod lexer {
    pub use m6lexerkit::*;
    // explicitly re-export override glob implicitly re-export
    pub use crate::{ make_char_matcher_rules, make_token_matcher_rules};
}
