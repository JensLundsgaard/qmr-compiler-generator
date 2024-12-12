use qmrl::{ast::*, emit::write_to_file, structures};

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
            realize: GateImplementationExpr::Unit,
        },
        trans: TransitionBlock {
            data: NamedTuple {
                name: "Swap".to_string(),
                fields: vec![
                    ("u".to_string(), Ty::LocationTy),
                    ("v".to_string(), Ty::LocationTy),
                ],
            },
            apply: Expr::SwapPair(
                Box::new(Expr::GetData {
                    d: DataType::Transition,
                    field: "u".to_string(),
                }),
                Box::new(Expr::GetData {
                    d: DataType::Transition,
                    field: "v".to_string(),
                }),
            ),
            cost: Expr::FloatLiteral(0.0),
        },
        arch: None,
    }
}

fn main() {
    let p = test_program();
    write_to_file(p, "test.rs");
}
