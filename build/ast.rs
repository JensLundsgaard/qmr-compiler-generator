


#[derive(Debug)]
pub  struct ProblemDefinition{
    pub imp : ImplBlock,
    pub trans : TransitionBlock,
    pub arch : Option<ArchitectureBlock>,
}
#[derive(Debug)]
pub struct ImplBlock{
   pub data : NamedTuple,
   pub realize : Expr,

}

#[derive(Debug)]
pub struct ArchitectureBlock{
    pub data : NamedTuple,
    
}

#[derive(Debug)]
pub struct TransitionBlock{
    pub data : NamedTuple,
    pub apply : Expr,
    pub cost : Expr,
    pub get_transitions : Expr
}
#[derive(Debug)]
pub struct NamedTuple{
    pub name : String,
    pub fields : Vec<(String, Ty)>
}
#[derive(Debug)]
pub enum Ty{
    LocationTy,
    TupleTy(Vec<Ty>)
}

#[derive(Debug)]
pub enum Expr{
    
    FloatLiteral(f64),
    LocationLiteral(usize),
    IndexLiteral(usize),
    Ident(String),

    Tuple(Vec<Expr>),

    SomeExpr(Box<Expr>),
    NoneExpr,
    
    SwapPair(Box<Expr>, Box<Expr>),
    GetData{d : DataType, access : AccessExpr},
    CallMethod{d : DataType, method : String, args : Vec<Expr>},
    ITE{
        cond : Box<Expr>,
        then : Box<Expr>,
        els : Box<Expr>
    },
    MapIterExpr{container : Box<Expr>, func : Box<Expr>},

    Append{vec : Box<Expr>, elem : Box<Expr>},
    
    TransitionConstructor(Vec<(String, Expr)>),
    ImplConstructorExpr(Vec<(String, Expr)>),
    

    MapAccess(Box<Expr>),

    Equal(Box<Expr>, Box<Expr>),

}
#[derive(Debug)]
pub enum AccessExpr{
    TupleAccess(String, Box<Expr>),
    ArrayAccess(String, Box<Expr>),
    Field(String)
}
#[derive(PartialEq, Debug, Clone)]
pub enum DataType {
    Arch, 
    Transition,
    Step, 
    Impl,
    Gate,
}

pub enum Context{
    DataTypeContext(DataType),
    Free
}

pub enum TransitionCostExpr{
    Unit
}