use chumsky::prelude::*;
use text::keyword;

use crate::{ast, ProblemDefinition};

fn type_parser() -> impl Parser<char, ast::Ty, Error = Simple<char>> {
    let atom_ty = just("Location").map(|_| ast::Ty::LocationTy);
    let tuple_ty = atom_ty
        .separated_by(just(","))
        .at_least(1)
        .delimited_by(just("("), just(")"))
        .map(ast::Ty::TupleTy);
    atom_ty.or(tuple_ty)
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

fn impl_block_parser() -> impl Parser<char, ast::ImplBlock, Error = Simple<char>> {
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
        .ignore_then(data)
        .padded()
        .then(realize)
        .padded()
        .then_ignore(just("]"))
        .padded()
        .map(|(data, realize)| ast::ImplBlock { data, realize })
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
    named_tuple_parser()
        .or_not()
        .map(|data| data.map(|d| ast::ArchitectureBlock { data: d }))
}

fn data_type_parser() -> impl Parser<char, ast::DataType, Error = Simple<char>> {
    keyword("Arch")
        .map(|_| ast::DataType::Arch)
        .or(keyword("Transition").map(|_| ast::DataType::Transition))
        .or(keyword("Impl").map(|_| ast::DataType::Impl))
        .or(keyword("Gate").map(|_| ast::DataType::Gate))
}

fn expr_parser() -> impl Parser<char, ast::Expr, Error = Simple<char>> {
    recursive(|expr_parser| {
        let float_literal = float_parser().map(ast::Expr::FloatLiteral).boxed();
        let location_literal = just("Location")
            .ignore_then(text::int(10).delimited_by(just("("), just(")")))
            .map(|i: String| ast::Expr::LocationLiteral(i.parse().unwrap()));
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

        let map_iter = just("map(|x| -> ")
            .padded()
            .ignore_then(expr_parser.clone())
            .then_ignore(just(",").padded())
            .then(expr_parser.clone())
            .then_ignore(just(")"))
            .map(|(func, container)| ast::Expr::MapIterExpr {
                container: Box::new(container),
                func: Box::new(func),
            });

        let container_atom = choice((
            ident,
            map_iter.clone(),
            expr_parser.clone().delimited_by(just("("), just(")")),
        ));
        let append = container_atom
            .then_ignore(just(".push"))
            .then(expr_parser.clone().delimited_by(just("("), just(")")))
            .map(|(vec, elem)| ast::Expr::Append {
                vec: Box::new(vec),
                elem: Box::new(elem),
            });

        let atom = choice((
            float_literal.clone(),
            location_literal.clone(),
            ident.clone(),
            tuple.clone(),
            expr_parser.clone().delimited_by(just("("), just(")")),
        ));
        let equality_comparison = atom
            .then_ignore(just("==").padded())
            .then(expr_parser.clone())
            .map(|(a, b)| ast::Expr::Equal(Box::new(a), Box::new(b)));
        let field = text::ident().map(|name| ast::AccessExpr::Field(name));

        // Define the tuple access suffix
        let tuple_access = text::ident()
            .then_ignore(just('.'))
            .then(expr_parser.clone())
            .map(|(f, e)| ast::AccessExpr::TupleAccess(f, Box::new(e)));

        // Define the array access suffix
        let array_access = text::ident()
            .then(expr_parser.clone().delimited_by(just('['), just(']')))
            .map(|(f, e)| ast::AccessExpr::ArrayAccess(f, Box::new(e)));

        let access_expr = choice((tuple_access, array_access, field));
        let get_data = data_type_parser()
            .then_ignore(just("."))
            .then(access_expr)
            .map(|(d, access)| ast::Expr::GetData { d, access });
        let map_access = just("Step.Map")
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

        let expr = choice((
            equality_comparison,
            ite,
            map_access,
            call_method,
            get_data,
            trans_cons,
            impl_cons,
            append,
            swap_pair,
            map_iter,
            float_literal,
            location_literal,
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
    let prob_def = {
        impl_block
            .then(transition_block)
            .then(architecture_block)
            .map(
                |((impl_block, transition_block), architecture_block)| ProblemDefinition {
                    imp: impl_block,
                    trans: transition_block,
                    arch: architecture_block,
                },
            )
    };
    prob_def
}

pub(crate) fn read_file(filename: &str) -> ProblemDefinition {
    let src = std::fs::read_to_string(filename).expect("Failed to read file");

    return parser()
        .parse(src)
        .expect("Failed to parse problem definition");
}
