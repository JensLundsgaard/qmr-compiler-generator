use std::panic::Location;

use crate::structures;


pub  struct ProblemDefinition{
    pub imp : ImplBlock,
    pub trans : TransitionBlock,
    pub arch : Option<ArchitectureBlock>,
}

pub struct ImplBlock{
   pub data : NamedTuple,
   pub realize : GateImplementationExpr,

}

pub struct ArchitectureBlock{
    pub data : NamedTuple,
    
}

pub struct TransitionBlock{
    pub data : NamedTuple,
    pub apply : Expr,
    pub cost : Expr,
}

pub struct NamedTuple{
    pub name : String,
    pub fields : Vec<(String, Ty)>
}

pub enum Ty{
    LocationTy
}

pub enum GateFilterExpr{
    IsType(structures::GateType),
    And(Box<GateFilterExpr>, Box<GateFilterExpr>),
    Or(Box<GateFilterExpr>, Box<GateFilterExpr>),
    Not(Box<GateFilterExpr>),
}

pub enum GateImplementationExpr{
    Unit
}

pub enum Expr{
    SwapPair(Box<Expr>, Box<Expr>),
    GetData{d : DataType, field : String},
    FloatLiteral(f64),
    ITE{
        cond : Box<Expr>,
        then : Box<Expr>,
        els : Box<Expr>
    },
    Unit

}
#[derive(PartialEq)]
pub enum DataType {
    Arch, 
    Transition,
    Step, 
    Impl
}
pub enum TransitionCostExpr{
    Unit
}