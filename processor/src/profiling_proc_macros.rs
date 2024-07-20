extern crate proc_macro;
use proc_macro::{ TokenStream, TokenTree };
use std::str::FromStr;

const MAX_PROFILE_SECTIONS: usize = 4096;
static mut PROFILE_COUNT: usize = 0;

#[proc_macro]
pub fn profile(input: TokenStream) -> TokenStream {
    #![allow(unused_variables)]

    // TODO when done making this function, remove the clone of input
    let mut token_tree_iterator = input.clone().into_iter();
    let section_label = match (token_tree_iterator.next(), token_tree_iterator.next()) {
        (Some(TokenTree::Literal(literal)), Some(TokenTree::Punct(punct))) => {
            let raw_literal = format!("{literal}");
            if !(raw_literal.starts_with('"') || raw_literal.starts_with("r#\"")) {
                panic!("expected string literal label but got {}", raw_literal);
            }

            if punct.as_char() != ';' {
                panic!("expected semicolon (;) delimiter after label");
            }

            raw_literal
        },
        _ => panic!("invalid macro invocation! expected \"str_lit; exprs...\""),
    };
    let original_code = token_tree_iterator;

    let profile_index = unsafe { PROFILE_COUNT };
    unsafe { PROFILE_COUNT += 1; }
    if unsafe { PROFILE_COUNT } > MAX_PROFILE_SECTIONS {
        panic!("Max profile sections ({}) exceeded!", MAX_PROFILE_SECTIONS);
    }

    let profile_section_begin = TokenStream::from_str(format!(r#"
        {{
            use performance_metrics::__GLOBAL_PROFILER;
            unsafe {{
                __GLOBAL_PROFILER.begin_section_profile({section_label}, {profile_index});
            }}
        }}
    "#).as_str());
    let profile_section_end = TokenStream::from_str(format!(r#"
        {{
             use performance_metrics::__GLOBAL_PROFILER;
             unsafe {{
                 __GLOBAL_PROFILER.end_section_profile({profile_index});
             }}
        }}
    "#).as_str());
    
    let mut instrumented_code = TokenStream::new();
    instrumented_code.extend(profile_section_begin);
    instrumented_code.extend(original_code);
    instrumented_code.extend(profile_section_end);

    instrumented_code
}

#[proc_macro]
pub fn __get_max_profile_sections(_: TokenStream) -> TokenStream {
    TokenStream::from(
        TokenTree::Literal(proc_macro::Literal::usize_suffixed(MAX_PROFILE_SECTIONS))
    )
}
