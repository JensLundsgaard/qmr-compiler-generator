use std::collections::binary_heap;

use chumsky::prelude::*;
use text::keyword;

use crate::{
    ast::{self, GateType},
    ProblemDefinition,
};

fn type_parser() -> impl Parser<char, ast::Ty, Error = Simple<char>> {
    recursive(|type_parser| {
        let atom_ty = just("Location")
            .map(|_| ast::Ty::LocationTy)
            .or(just("Int").map(|_| ast::Ty::IntTy))
            .or(just("Float").map(|_| ast::Ty::FloatTy));

        let tuple_ty = type_parser
            .clone()
            .separated_by(just(","))
            .at_least(1)
            .delimited_by(just("("), just(")"))
            .map(ast::Ty::TupleTy);

        let vector_ty = just("Vec")
            .ignore_then(just("<"))
            .ignore_then(type_parser.clone())
            .then_ignore(just(">"))
            .map(|v| ast::Ty::VectorTy(Box::new(v)));

        atom_ty.or(tuple_ty).or(vector_ty)
    })
}

fn named_tuple_parser() -> impl Parser<char, ast::NamedTuple, Error = Simple<char>> {
    let name = {
        text::keyword("name")
            .padded()
            .ignore_then(just("="))
            .padded()
            .ignore_then(text::ident().delimited_by(just("'"), just("'")))
            .padded()
    };
    let fields = {
        text::keyword("data")
            .padded()
            .ignore_then(just("="))
            .padded()
            .ignore_then(
                (text::ident()
                    .padded()
                    .then_ignore(just(":"))
                    .padded()
                    .then(type_parser())
                    .padded())
                .separated_by(just(",").padded())
                .at_least(1)
                .delimited_by(just("("), just(")")),
            )
            .padded()
            .map(|fields| fields.into_iter().collect())
    };
    name.padded()
        .then(fields)
        .map(|(name, fields)| ast::NamedTuple { name, fields })
}

fn float_parser() -> impl Parser<char, f64, Error = Simple<char>> {
    let sign = just('-').or_not().map(|s| -> f64 {
        if s.is_some() {
            -1.0
        } else {
            1.0
        }
    });
    let digits = text::int(10).padded();
    let decimal = just('.')
        .then(text::digits(10))
        .map(|(_, d)| format!(".{}", d));
    sign.then(digits)
        .then(decimal)
        .map(|((s, int), digits)| s * format!("{}{}", int, digits).parse::<f64>().unwrap())
}

fn gate_type_parser() -> impl Parser<char, Vec<ast::GateType>, Error = Simple<char>> {
    let gate_type = just("CX")
        .map(|_| ast::GateType::CX)
        .or(just("T").map(|_| ast::GateType::T))
        .or(just("Pauli").map(|_| ast::GateType::Pauli));
    gate_type.separated_by(just(",").padded()).at_least(1)
}

fn bin_op_parser() -> impl Parser<char, ast::BinOp, Error = Simple<char>>{
     just("==")
        .map(|_| ast::BinOp::Equals)
        .or(just("/").map(|_| ast::BinOp::Div))
        .or(just("*").map(|_| ast::BinOp::Mult))
        .or(just("-").map(|_| ast::BinOp::Minus))
        .or(just("+").map(|_| ast::BinOp::Plus))
    
}

fn impl_block_parser() -> impl Parser<char, ast::ImplBlock, Error = Simple<char>> {
    let routed_gates = {
        text::keyword("routed_gates")
            .padded()
            .ignore_then(just("="))
            .padded()
            .ignore_then(gate_type_parser())
            .padded()
    };
    let data = named_tuple_parser();
    let realize = keyword("realize_gate")
        .padded()
        .ignore_then(just("="))
        .padded()
        .ignore_then(expr_parser())
        .padded();
    keyword("GateRealization")
        .padded()
        .then_ignore(just("["))
        .padded()
        .ignore_then(routed_gates)
        .padded()
        .then(data)
        .padded()
        .then(realize)
        .padded()
        .then_ignore(just("]"))
        .padded()
        .map(|((routed_gates, data), realize)| ast::ImplBlock { routed_gates, data, realize })
}

fn trans_block_parser() -> impl Parser<char, ast::TransitionBlock, Error = Simple<char>> {
    let data = named_tuple_parser();
    let get_transitions = just("get_transitions")
        .padded()
        .ignore_then(just("="))
        .padded()
        .ignore_then(expr_parser())
        .padded();
    let apply = just("apply")
        .padded()
        .ignore_then(just("="))
        .padded()
        .ignore_then(expr_parser())
        .padded();
    let cost = just("cost")
        .padded()
        .ignore_then(just("="))
        .padded()
        .ignore_then(expr_parser())
        .padded();
    keyword("Transition")
        .padded()
        .then_ignore(just("["))
        .padded()
        .ignore_then(data)
        .padded()
        .then(get_transitions)
        .padded()
        .then(apply)
        .padded()
        .then(cost)
        .padded()
        .then_ignore(just("]"))
        .padded()
        .map(
            |(((data, get_transitions), apply), cost)| ast::TransitionBlock {
                data,
                get_transitions,
                apply,
                cost,
            },
        )
}

fn method_name() -> impl Parser<char, String, Error = Simple<char>> {
    let first_char = filter(|c: &char| c.is_alphabetic() || *c == '_');
    let rest = filter(|c: &char| c.is_alphanumeric() || *c == '_');
    first_char
        .then(rest.repeated())
        .map(|(f, r)| format!("{}{}", f, r.iter().collect::<String>()))
}

fn arch_block_parser() -> impl Parser<char, Option<ast::ArchitectureBlock>, Error = Simple<char>> {
    let data = named_tuple_parser();
    let get_locations = just("get_locations")
        .padded()
        .ignore_then(just("="))
        .padded()
        .ignore_then(expr_parser())
        .padded();
    keyword("Architecture")
        .padded()
        .then_ignore(just("["))
        .padded()
        .ignore_then(data)
        .padded()
        .then(get_locations.or_not())
        .padded()
        .then_ignore(just("]"))
        .map(|(data, get_locations)| ast::ArchitectureBlock {
            data,
            get_locations,
        })
        .or_not()
}

fn step_block_parser() -> impl Parser<char, Option<ast::StepBlock>, Error = Simple<char>> {
    let cost = just("cost")
        .padded()
        .ignore_then(just("="))
        .ignore_then(expr_parser())
        .padded();
    keyword("Step")
        .padded()
        .then_ignore(just("["))
        .padded()
        .ignore_then(cost)
        .padded()
        .then_ignore(just("]"))
        .map(|cost| ast::StepBlock { cost })
        .or_not()
}

fn data_type_parser() -> impl Parser<char, ast::DataType, Error = Simple<char>> {
    keyword("Arch")
        .map(|_| ast::DataType::Arch)
        .or(keyword("Transition").map(|_| ast::DataType::Transition))
        .or(keyword("Impl").map(|_| ast::DataType::Impl))
        .or(keyword("Gate").map(|_| ast::DataType::Gate))
        .or(keyword("Step").map(|_| ast::DataType::Step))
}

fn expr_parser() -> impl Parser<char, ast::Expr, Error = Simple<char>> {
    recursive(|expr_parser| {
        let float_literal = float_parser().map(ast::Expr::FloatLiteral).boxed();
        let location_literal = just("Location")
            .ignore_then(text::int(10).delimited_by(just("("), just(")")))
            .map(|i: String| ast::Expr::LocationLiteral(i.parse().unwrap()));
        let empty_vec = just("Vec")
            .ignore_then(just("()"))
            .map(|_| ast::Expr::EmptyVec);
        let index_literal =
            text::int(10).map(|i: String| ast::Expr::IndexLiteral(i.parse().unwrap()));
        let ident = text::ident().map(ast::Expr::Ident);
        let tuple = expr_parser
            .clone()
            .separated_by(just(",").padded())
            .at_least(1)
            .delimited_by(just("("), just(")"))
            .map(ast::Expr::Tuple);
        let some_expr = just("Some")
            .ignore_then(expr_parser.clone().delimited_by(just("("), just(")")))
            .map(|expr: ast::Expr| ast::Expr::SomeExpr(Box::new(expr)));
        let none_expr = just("None").map(|_| ast::Expr::NoneExpr);
        let swap_pair = keyword("value_swap")
            .ignore_then(just("("))
            .ignore_then(expr_parser.clone())
            .then_ignore(just(",").padded())
            .then(expr_parser.clone())
            .then_ignore(just(")"))
            .map(|(a, b)| ast::Expr::SwapPair(Box::new(a), Box::new(b)));

        let map_iter = just("map(|")
            .ignore_then(text::ident())
            .then_ignore(just("| ->"))
            .padded()
            .then(expr_parser.clone())
            .then_ignore(just(",").padded())
            .then(expr_parser.clone())
            .then_ignore(just(")").padded())
            .map(|((ident, func), container)| ast::Expr::MapIterExpr {
                container: Box::new(container),
                bound_var: ident,
                func: Box::new(func),
            });
        let fold = just("fold(")
            .padded()
            .ignore_then(expr_parser.clone().padded())
            .then_ignore(just(",").padded()) 
            .then_ignore(just("|x, acc| ->").padded())
            .then(expr_parser.clone())
            .then_ignore(just(",").padded())
            .then(expr_parser.clone())
            .then_ignore(just(")"))
            .map(|((init, func), container)| ast::Expr::FoldExpr {
                container: Box::new(container),
                init: Box::new(init),
                func: Box::new(func),
            });

        let container_atom = choice((
            ident,
            map_iter.clone(),
            expr_parser.clone().delimited_by(just("("), just(")")),
        ));
        let append = container_atom
            .clone()
            .then_ignore(just(".push").padded())
            .then(expr_parser.clone().delimited_by(just("("), just(")")))
            .map(|(vec, elem)| ast::Expr::Append {
                vec: Box::new(vec),
                elem: Box::new(elem),
            });

        let extend = container_atom
            .clone()
            .then_ignore(just(".extend").padded())
            .then(expr_parser.clone().delimited_by(just("("), just(")")))
            .map(|(vec, elem)| ast::Expr::Extend {
                vec1: Box::new(vec),
                vec2: Box::new(elem),
            });
        let access_chain = recursive(|access_chain_parser| {
            let array_access = expr_parser
                .clone()
                .delimited_by(just('['), just(']'))
                .then(access_chain_parser.clone())
                .map(|(expr, access_chain)| {
                    ast::AccessChain::ArrayAccess(Box::new(expr), access_chain)
                });
            let tuple_access = just('.')
                .ignore_then(expr_parser.clone().delimited_by(just("("), just(")")))
                .then(access_chain_parser.clone())
                .map(|(expr, access_chain)| {
                    ast::AccessChain::TupleAccess(Box::new(expr), access_chain)
                });
            (array_access.or(tuple_access))
                .or_not()
                .map(|c| Box::new(c.unwrap_or(ast::AccessChain::Nil)))
        });
        let access_expr = text::ident()
            .then(access_chain)
            .map(|(id, ac)| ast::AccessExpr::Access(id, ac));
        let get_data = data_type_parser()
            .then_ignore(just("."))
            .then(access_expr.clone())
            .map(|(d, access)| ast::Expr::GetData { d, access });

        let get_anon_data = text::ident()
            .then_ignore(just("."))
            .then(access_expr.clone())
            .map(|(ident, access)| ast::Expr::GetAnonData { ident, access });

        let map_access = just("Step.map")
            .ignore_then(just("["))
            .ignore_then(expr_parser.clone())
            .then_ignore(just("]"))
            .map(|x| ast::Expr::MapAccess(Box::new(x)));

        let call_method = data_type_parser()
            .then_ignore(just("."))
            .then(method_name())
            .then_ignore(just("("))
            .then(expr_parser.clone().separated_by(just(",").padded()))
            .then_ignore(just(")"))
            .map(|((d, method), args)| ast::Expr::CallMethod { d, method, args });

        let call_function = method_name()
            .then_ignore(just("(").padded())
            .then(expr_parser.clone().separated_by(just(",").padded()))
            .then_ignore(just(")").padded())
            .map(|(func, args)| ast::Expr::CallFunction { func, args });

        let ite = keyword("if")
            .padded()
            .ignore_then(expr_parser.clone())
            .padded()
            .then_ignore(keyword("then"))
            .padded()
            .then(expr_parser.clone())
            .padded()
            .then_ignore(keyword("else"))
            .padded()
            .then(expr_parser.clone())
            .padded()
            .map(|((cond, then), els)| ast::Expr::ITE {
                cond: Box::new(cond),
                then: Box::new(then),
                els: Box::new(els),
            });

        let some_arm = just("Some")
            .ignore_then(text::ident().delimited_by(just("("), just(")")))
            .ignore_then(just("=>"))
            .ignore_then(expr_parser.clone());

        let none_arm = just("None")
            .ignore_then(just("=>").padded())
            .ignore_then(expr_parser.clone());

        let option_match_some_first = just("match")
            .ignore_then(expr_parser.clone().padded())
            .then_ignore(just("{"))
            .then(some_arm.clone().padded())
            .then_ignore(just(","))
            .then(none_arm.clone().padded())
            .then_ignore(just("}"))
            .map(|((expr, some_arm), none_arm)| ast::Expr::OptionMatch {
                expr: Box::new(expr),
                some_arm: Box::new(some_arm),
                none_arm: Box::new(none_arm),
            });
        let option_match_none_first = just("match")
            .ignore_then(expr_parser.clone())
            .then_ignore(just("{"))
            .then(none_arm.clone().padded())
            .then_ignore(just(","))
            .then(some_arm.clone().padded())
            .then_ignore(just("}"))
            .map(|((expr, some_arm), none_arm)| ast::Expr::OptionMatch {
                expr: Box::new(expr),
                some_arm: Box::new(some_arm),
                none_arm: Box::new(none_arm),
            });
        let option_match = choice((option_match_some_first, option_match_none_first));
        let assign = just("=").padded();
        let assignment_parser = text::ident()
            .padded()
            .then_ignore(assign)
            .then(expr_parser.clone())
            .separated_by(just(",").padded())
            .at_least(1)
            .map(|assignments| assignments.into_iter().collect::<Vec<_>>());

        let trans_cons = keyword("Transition")
            .padded()
            .ignore_then(just("{").padded())
            .ignore_then(assignment_parser.clone())
            .then_ignore(just("}").padded())
            .map(ast::Expr::TransitionConstructor);

        let impl_cons = keyword("GateRealization")
            .padded()
            .ignore_then(just("{").padded())
            .ignore_then(assignment_parser.clone())
            .then_ignore(just("}").padded())
            .map(ast::Expr::ImplConstructorExpr);

        let atom = choice((
            float_literal.clone(),
            location_literal.clone(),
            ident.clone(),
            tuple.clone(),
            expr_parser.clone().delimited_by(just("("), just(")")),
        ));
        let bin_op = atom
            .then(bin_op_parser().padded())
            .then(expr_parser.clone())
            .map(|((a, op), b)| ast::Expr::BinOp(op, Box::new(a), Box::new(b)));
        let expr = choice((
            bin_op,
            ite,
            option_match,
            map_access,
            call_method,
            empty_vec,
            get_data,
            trans_cons,
            impl_cons,
            append,
            extend,
            get_anon_data,
            swap_pair,
            map_iter,
            fold,
            float_literal,
            location_literal,
            call_function,
            index_literal,
            some_expr,
            none_expr,
            tuple,
            ident,
        ));
        expr
    })
}

fn parser() -> impl Parser<char, ProblemDefinition, Error = Simple<char>> {
    let impl_block = impl_block_parser();
    let transition_block = trans_block_parser();
    let architecture_block = arch_block_parser();
    let step_block = step_block_parser();
    let prob_def = {
        impl_block
            .then(transition_block)
            .then(architecture_block)
            .then(step_block)
            .map(
                |(((impl_block, transition_block), architecture_block), step_block)| {
                    ProblemDefinition {
                        imp: impl_block,
                        trans: transition_block,
                        arch: architecture_block,
                        step: step_block,
                    }
                },
            )
    };
    prob_def
}

pub(crate) fn read_file(filename: &str) -> ProblemDefinition {
    let src = std::fs::read_to_string(filename).expect("Reading qmrl file");
    println!("{:?}", parser().parse(src.clone()).unwrap());
    return parser()
        .parse(src)
        .expect("Failed to parse problem definition");
}
