extern crate proc_macro;
use proc_macro::{ Group, Delimiter, TokenStream, TokenTree };
use std::{ str::FromStr, iter };

const MAX_PROFILE_SECTIONS: usize = 4096;
static mut PROFILE_COUNT: usize = 0;

// TODO 2 options here:
// 1) Make this way smarter (possibly using the syn crate) so that it can decide when to properly
//    inject the manual drop at the end of the block.
// 2) Force the user to specify if a manual drop is needed at the end.
fn instrument_code(label: &str, code: proc_macro::token_stream::IntoIter) -> TokenStream {
    // FIXME probably big perf hit, but maybe okay since it's just compile-time.
    let code = code.collect::<Vec<TokenTree>>();
    if code.is_empty() {
        panic!("Empty profile section");
    }

    // FIXME hate this
    let ends_with_semicolon = code[code.len() - 1].to_string() == ";";
    let ends_with_profile_macro = code[code.len() - 3].to_string() == "profile" && code[code.len() - 2].to_string() == "!";

    let index = unsafe { PROFILE_COUNT };
    unsafe { PROFILE_COUNT += 1 };
    if unsafe { PROFILE_COUNT } >= MAX_PROFILE_SECTIONS {
        panic!("Exceeded max profile section count of {}", MAX_PROFILE_SECTIONS);
    }

    let autoprofile_varname = format!("__auto_profile_{index}");
    let profile_section_begin = TokenStream::from_str(format!(r#"
        let {autoprofile_varname} = crate::performance_metrics::AutoProfile::new({label}, {index});
    "#).as_str());

    let mut instrumented_code = TokenStream::new();
    instrumented_code.extend(profile_section_begin);
    instrumented_code.extend(code);
    if ends_with_semicolon || ends_with_profile_macro {
        instrumented_code.extend(TokenStream::from_str(format!("drop({autoprofile_varname});").as_str()));
    }

    instrumented_code
}

#[proc_macro]
pub fn profile(input: TokenStream) -> TokenStream {
    #![allow(unused_variables)]

    let mut token_tree_iterator = input.into_iter();
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

    instrument_code(&section_label, token_tree_iterator)
}

// FIXME this may not work with visibility modifiers
#[proc_macro_attribute]
pub fn profile_function(attribute: TokenStream, function: TokenStream) -> TokenStream {
    let mut attribute_token_tree = attribute.into_iter();
    let mut function_token_tree = function.into_iter().peekable();

    // TODO do we need to be collecting and emitting these with the rest of the instrumented code?
    // advance the token tree iterator past other attributes
    while matches!(function_token_tree.peek(), Some(TokenTree::Punct(punct)) if punct.to_string() == "#") {
        function_token_tree.next(); // skip "#"
        function_token_tree.next(); // skip the attribute group [ ... ]
    }

    assert!(
        matches!(function_token_tree.next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "fn"),
        "This attribute proc macro can only be attached to functions"
    );

    // At this point we've already checked the first token in the function ("fn"), so the next
    // token MUST be the function name.
    let function_name = function_token_tree.next().expect("expected function name");
    let mut function_signature: Vec<TokenTree> = Vec::with_capacity(32);
    while !matches!(function_token_tree.peek(), Some(TokenTree::Group(group)) if matches!(group.delimiter(), Delimiter::Brace)) {
        function_signature.push(function_token_tree.next().unwrap());
    }
    let function_body = function_token_tree.next().expect("expected function body");

    let section_label = match attribute_token_tree.next() {
        Some(TokenTree::Literal(literal)) => {
            let raw_literal = format!("{literal}");
            if !(raw_literal.starts_with('"') || raw_literal.starts_with("r#\"")) {
                panic!("expected string literal label but got {}", raw_literal);
            }

            raw_literal.trim_matches('"').to_string()
        },
        None => format!(r#""{function_name}""#),
        _ => panic!("Expected attribute to contain a string literal"),
    };

    // The last token in the function token stream should be a group containing the function body.
    let instrumented_function_body = match function_body {
        TokenTree::Group(group) => instrument_code(&section_label, group.stream().into_iter()),
        _ => panic!("expected a group for the function body but got something else"),
    };

    let mut instrumented = TokenStream::new();
    instrumented.extend(TokenStream::from_str("fn"));
    instrumented.extend(iter::once(function_name));
    instrumented.extend(function_signature);
    instrumented.extend(iter::once(TokenTree::Group(Group::new(Delimiter::Brace, instrumented_function_body))));

    instrumented
}

#[proc_macro]
pub fn __get_max_profile_sections(_: TokenStream) -> TokenStream {
    TokenStream::from(
        TokenTree::Literal(proc_macro::Literal::usize_suffixed(MAX_PROFILE_SECTIONS))
    )
}
