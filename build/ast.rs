



pub  struct ProblemDefinition{
    pub imp : ImplBlock,
    pub trans : TransitionBlock,
    pub arch : Option<ArchitectureBlock>,
}

pub struct ImplBlock{
   pub data : NamedTuple,
   pub realize : Expr,

}

pub struct ArchitectureBlock{
    pub data : NamedTuple,
    
}

pub struct TransitionBlock{
    pub data : NamedTuple,
    pub apply : Expr,
    pub cost : Expr,
    pub get_transitions : Expr
}

pub struct NamedTuple{
    pub name : String,
    pub fields : Vec<(String, Ty)>
}

pub enum Ty{
    LocationTy,
    TupleTy(Vec<Ty>)
}


pub enum Expr{
    SwapPair(Box<Expr>, Box<Expr>),
    GetData{d : DataType, access : AccessExpr},
    CallMethod{d : DataType, method : String, args : Vec<Expr>},
    FloatLiteral(f64),
    LocationLiteral(usize),
    ITE{
        cond : Box<Expr>,
        then : Box<Expr>,
        els : Box<Expr>
    },
    Append{vec : Box<Expr>, elem : Box<Expr>},
    TransitionConstructor(Vec<(String, Expr)>),
    ImplConstructorExpr(Vec<(String, Expr)>),
    Tuple(Vec<Expr>),
    MapAccess(Box<Expr>),
    QubitAccess(usize),
    MapIterExpr{container : Box<Expr>, func : Box<Expr>},
    SomeExpr(Box<Expr>),
    NoneExpr,
    VarExpr(String),
    Equal(Box<Expr>, Box<Expr>),

}
pub enum AccessExpr{
    IndexInto(Box<AccessExpr>, usize),
    Field(String)
}
#[derive(PartialEq)]
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