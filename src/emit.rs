use crate::ast::*;
use crate::structures;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn emit_program(p: ProblemDefinition) -> TokenStream {
    let use_statements = quote! {
        use qmrl::*;
        use petgraph::{graph::NodeIndex, Graph};
    };
    let define_gi_struct = emit_define_struct(&p.imp.data);
    let define_arch_struct = emit_define_arch_struct(&p.arch);
    let define_transition_struct = emit_define_struct(&p.trans.data);
    let implement_gi_trait = emit_impl_gate(&p.imp.data);
    let implement_arch_trait = emit_impl_arch(&p.arch);
    let implement_trans_trait = emit_impl_trans(&p.trans, &p.imp);
    quote! {
        #use_statements
        #define_gi_struct
        #define_arch_struct
        #define_transition_struct
        #implement_gi_trait
        #implement_arch_trait
        #implement_trans_trait

    }
}

fn emit_define_struct(data: &NamedTuple) -> TokenStream {
    let struct_name = syn::Ident::new(&data.name, Span::call_site());
    let fields = data.fields.iter().map(|(name, ty)| {
        let field_name = syn::Ident::new(name, Span::call_site());
        let field_ty : syn::Type  = match ty {
            Ty::LocationTy => syn::parse_quote!(Location),
        };
        quote! { #field_name : #field_ty }
    });
    quote! {
        #[derive(Debug)]
        pub struct #struct_name {
            #(#fields),*
        }
    }
}

fn emit_define_arch_struct(arch: &Option<ArchitectureBlock>) -> TokenStream {
    let extra_fields_quote = match arch {
        Some(ref arch) => {
            let extra_fields = arch.data.fields.iter().map(|(name, ty)| {
                let field_name = syn::Ident::new(name, Span::call_site());
                let field_ty = match ty {
                    Ty::LocationTy => syn::Ident::new("Location", Span::call_site()),
                };
                quote! { #field_name : #field_ty }
            });
            quote! {#(#extra_fields),*}
        }
        None => {
            quote! {}
        }
    };
    quote! {
            struct MyArch {
                graph: Graph<Location, ()>,
                index_map: HashMap<Location, NodeIndex>,
                #extra_fields_quote
            }
    }
}
fn emit_impl_gate(imp_data: &NamedTuple) -> TokenStream {
    let struct_name = syn::Ident::new(&imp_data.name, Span::call_site());
    quote! {impl GateImplementation for #struct_name {}}
}

fn emit_impl_arch(arch: &Option<ArchitectureBlock>) -> TokenStream {
    let struct_name = syn::Ident::new("MyArch", Span::call_site());
    let body = match arch {
        Some(arch) => {
            quote! {todo!()}
        }
        None => {
            quote! {
                    let mut locations = Vec::new();
                    for node in self.graph.node_indices() {
                        locations.push(self.graph[node]);
                    }
                    return locations;
            }
        }
    };
    return quote! {
    
    impl Architecture for #struct_name {
        fn get_locations(&self) -> Vec<Location>{
            #body
        }
    
    }};
}

fn emit_impl_trans(t : &TransitionBlock, imp : &ImplBlock) -> TokenStream{
    let trans_struct_name = syn::Ident::new(&t.data.name, Span::call_site());
    let imp_struct_name = syn::Ident::new(&imp.data.name, Span::call_site());
    let apply_expr = emit_expr(&t.apply, &DataType::Transition);
    let cost_expr = emit_expr(&t.cost, &DataType::Transition);
    quote! {
        impl Transition<#imp_struct_name> for #trans_struct_name {
            fn apply(&self, step: &Step<#imp_struct_name>) -> Step<#imp_struct_name> {
               #apply_expr
            }

            fn repr(&self) -> String {
                return format!("{:?}", self);
            }

            fn cost(&self) -> f64 {
                #cost_expr
            }
        }


    }

}

fn emit_expr(e: &Expr, context : &DataType) -> TokenStream {
    match e {
        Expr::Unit => quote! {todo!()},
        Expr::SwapPair(left, right) => {
        let emit_left = emit_expr(left, context);
        let emit_right = emit_expr(right, context);
        quote! {
            let mut new_step = step.clone();
            let left = #emit_left;
            let right = #emit_right;
            new_step.map = utils::swap_keys(&step.map, left, right);
            return new_step;
        }
    }
        Expr::GetData { d, field } => {
            let field_name = syn::Ident::new(field, Span::call_site());
            let data_name = 
                if context == d {
                    syn::Ident::new("self", Span::call_site())
                } else {
                    match d {
                        DataType::Arch =>  syn::Ident::new("arch", Span::call_site()),
                        DataType::Transition => syn::Ident::new("t", Span::call_site()),
                        DataType::Step => syn::Ident::new("step", Span::call_site()),
                        DataType::Impl => syn::Ident::new("gi", Span::call_site()),
                }
                };
           
            quote! {
                #data_name.#field_name
            }
        }
        Expr::FloatLiteral(n) => quote! {#n},
        Expr::ITE { cond, then, els } => todo!(),
    }
}

pub fn write_to_file(p: ProblemDefinition, filename: &str) {
    let s: TokenStream = emit_program(p);
    let raw_str = s.to_string();
    let parse_res = syn::parse2(s);
    let formatted = match parse_res {
        Ok(syntax_tree) => prettyplease::unparse(&syntax_tree),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            raw_str
        }
    };
    let _ = std::fs::write(filename, formatted.as_bytes());
}
