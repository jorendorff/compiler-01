#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug)]
pub enum Expr {
    IntLit(i64),
    Var(String),
    UnaryMinus(Box<Expr>),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug)]
pub enum Stmt {
    Let { name: String, expr: Expr },
    Assign { name: String, expr: Expr },
    Print { expr: Expr },
}
