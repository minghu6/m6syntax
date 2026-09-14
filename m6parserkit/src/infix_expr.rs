use m6stack::{Stack, stack};

pub trait Bop {
    fn precedence(&self) -> i32;
}


#[derive(Debug)]
pub struct BopWrapper<T> {
    value: Box<T>,
    prec: i32
}


impl<T> BopWrapper<T> {
    pub fn new(value: T, prec: i32) -> Self {
        Self {
            value: Box::new(value),
            prec
        }
    }

    pub fn unwrap(self) -> T {
        *self.value
    }
}

impl<T> Bop for BopWrapper<T> {
    fn precedence(&self) -> i32 {
        self.prec
    }
}


pub enum InfixExpr<O, T> {
    E(T),
    T {
        bop: O,
        pri1: Box<Self>,
        pri2: Box<Self>
    }
}

impl<O, T> InfixExpr<O, T> {
    fn combine(self, bop: O, pri2: Self) -> Self {
        Self::T { bop, pri1: Box::new(self), pri2: Box::new(pri2) }
    }
}


/// Return Reverse Polish Notation (RPN) form (or Post-order, LRN(left right node) traversal)
pub fn parse_infix_expr<O: Bop, T>(bops: Vec<O>, pris: Vec<T>) -> InfixExpr<O, T> {
    debug_assert_eq!(bops.len() + 1, pris.len());
    debug_assert!(bops.len() > 0);

    // Resort the Infix expression by precedence (they don't change left most expr srcloc)
    // reverse order
    let mut out_bop_stack = Stack::from(bops);
    let mut out_pri_stack = Stack::from(pris);

    let mut staging_bop_stack: Stack<O> = stack![];
    let mut expr_stack = stack![InfixExpr::E(out_pri_stack.pop().unwrap())];

    // out_bop_stack would be same with out_pri_stack in size.
    while !(out_bop_stack.is_empty() && staging_bop_stack.is_empty()) {
        // Reduce
        if out_bop_stack.is_empty()
            || !staging_bop_stack.is_empty()
                && staging_bop_stack.peek().unwrap().precedence()
                    // >= for left associative operator
                    >= out_bop_stack.peek().unwrap().precedence()
        {
            let bop = staging_bop_stack.pop().unwrap();
            let rhexpr = expr_stack.pop().unwrap();
            let lfexpr = expr_stack.pop().unwrap();

            expr_stack.push(lfexpr.combine(bop, rhexpr));
        }
        // Shift
        else {
            staging_bop_stack.push(out_bop_stack.pop().unwrap());
            expr_stack.push(InfixExpr::E(out_pri_stack.pop().unwrap()));
        }
    }

    expr_stack.pop().unwrap()
}
