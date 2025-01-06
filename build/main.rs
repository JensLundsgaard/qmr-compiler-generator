mod ast;
mod emit;
use std::{env, path::Path, vec};

use ast::*;
use emit::write_to_file;

fn test_program() -> ProblemDefinition {
    ProblemDefinition {
        imp: ImplBlock {
            data: NamedTuple {
                name: "NisqGateImplementation".to_string(),
                fields: vec![
                    ("u".to_string(), Ty::LocationTy),
                    ("v".to_string(), Ty::LocationTy),
                ],
            },
            realize: Expr::ITE {
                cond: Box::new(Expr::CallMethod {
                    d: DataType::Arch,
                    method: "contains_edge".to_string(),
                    args: vec![Expr::Tuple(vec![
                        Expr::MapAccess(Box::new(Expr::QubitAccess(0))),
                        Expr::MapAccess(Box::new(Expr::QubitAccess(1))),
                    ])],
                }),
                then: Box::new(Expr::SomeExpr(Box::new(Expr::ImplConstructorExpr(vec![
                    (
                        "u".to_string(),
                        Expr::MapAccess(Box::new(Expr::QubitAccess(0))),
                    ),
                    (
                        "v".to_string(),
                        Expr::MapAccess(Box::new(Expr::QubitAccess(1))),
                    ),
                ])))),
                els: Box::new(Expr::NoneExpr),
            },
        },
        trans: TransitionBlock {
            data: NamedTuple {
                name: "Swap".to_string(),
                fields: vec![(
                    "edge".to_string(),
                    Ty::TupleTy(vec![Ty::LocationTy, Ty::LocationTy]),
                )],
            },
            apply: Expr::SwapPair(
                Box::new(Expr::GetData {
                    d: DataType::Transition,
                    access: AccessExpr::IndexInto(
                        Box::new(AccessExpr::Field("edge".to_string())),
                        0,
                    ),
                }),
                Box::new(Expr::GetData {
                    d: DataType::Transition,
                    access: AccessExpr::IndexInto(
                        Box::new(AccessExpr::Field("edge".to_string())),
                        1,
                    ),
                }),
            ),
            cost: Expr::ITE {
                cond: Box::new(Expr::Equal(
                    Box::new(Expr::GetData {
                        d: DataType::Transition,
                        access: AccessExpr::Field("edge".to_string()),
                    }),
                    Box::new(Expr::Tuple(vec![
                        Expr::Tuple(vec![Expr::LocationLiteral(0), Expr::LocationLiteral(0)]),
                    ]))),
                ),
                then: Box::new(Expr::FloatLiteral(0f64)),
                els: Box::new(Expr::FloatLiteral(1f64)),
            },
            get_transitions: Expr::Append {
                vec: Box::new(Expr::MapIterExpr {
                    container: Box::new(Expr::CallMethod {
                        d: DataType::Arch,
                        method: "edges".to_string(),
                        args: vec![],
                    }),
                    func: Box::new(Expr::TransitionConstructor(vec![(
                        "edge".to_string(),
                        Expr::VarExpr("x".to_string()),
                    )])),
                }),
                elem: Box::new(Expr::TransitionConstructor(vec![(
                    "edge".to_string(),
                    Expr::Tuple(vec![Expr::LocationLiteral(0), Expr::LocationLiteral(0)]),
                )])),
            },
        },
        arch: None,
    }
}

fn main() {
    let p = test_program();
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("custom.rs");
    write_to_file(p, dest_path.to_str().unwrap());
}
