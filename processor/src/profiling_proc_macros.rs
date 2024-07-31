extern crate proc_macro;
use proc_macro::{ Group, Delimiter, TokenStream, TokenTree };
use std::{ str::FromStr, iter };

const MAX_PROFILE_SECTIONS: usize = 4096;
static mut PROFILE_COUNT: usize = 0;

// STRETCH GOAL
//
// Figure out a way to bake initialization of the profile sections array in the global profiler at
// compile time so that we only ever have as many profile sections as we actually need. Then we
// can change the array type from [Option<ProfileSection>; MAX_PROFILE_SECTIONS] to
// [ProfileSection; PROFILE_COUNT].

fn instrument_code<T>(label: &str, code: T, include_manual_drop: bool) -> TokenStream
where T: Iterator<Item = proc_macro::TokenTree> {
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
    if include_manual_drop {
        instrumented_code.extend(TokenStream::from_str(format!("drop({autoprofile_varname});").as_str()));
    }

    instrumented_code
}

#[proc_macro]
pub fn profile(input: TokenStream) -> TokenStream {
    #![allow(unused_variables)]

    let mut token_tree_iterator = input.into_iter().peekable();
    let section_label = match token_tree_iterator.next() {
        Some(TokenTree::Literal(literal)) => {
            let raw_literal = format!("{literal}");
            if !(raw_literal.starts_with('"') || raw_literal.starts_with("r#\"")) {
                panic!("expected string literal label but got {}", raw_literal);
            }

            raw_literal
        },
        _ => panic!("expected string literal label")
    };

    let include_manual_drop = match (token_tree_iterator.next(), token_tree_iterator.peek()) {
        (Some(TokenTree::Ident(ident)), Some(TokenTree::Punct(punct))) if punct.as_char() == ';' => {
            // we only peeked to the punct so advanced the token tree iterator
            token_tree_iterator.next();
            if ident.to_string() != "no_manual_drop" {
                panic!("unrecognized identifier, expected 'no_manual_drop'");
            }

            false
        },
        (Some(TokenTree::Punct(punct)), _) if punct.as_char() == ';' => true,

        _ => panic!("expected ident 'no_manual_drop' and/or semicolon")
    };

    instrument_code(&section_label, token_tree_iterator, include_manual_drop)
}

#[proc_macro_attribute]
pub fn profile_function(attribute: TokenStream, function: TokenStream) -> TokenStream {
    let mut attribute_token_tree = attribute.into_iter();
    let mut function_token_tree = function.into_iter().peekable();

    // All the stuff before the "fn" identifier, including other attributes, the visibility
    // modifier, etc.
    let mut function_decl_prelude: Vec<TokenTree> = Vec::with_capacity(64);
    while !matches!(function_token_tree.peek(), Some(TokenTree::Ident(ident)) if ident.to_string() == "fn") {
        function_decl_prelude.push(function_token_tree.next().unwrap());
    }

    function_token_tree.next(); // skip past "fn"
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

            raw_literal.to_string()
        },
        None => format!(r#""{function_name}""#),
        _ => panic!("Expected attribute to contain a string literal"),
    };

    // The last token in the function token stream should be a group containing the function body.
    let instrumented_function_body = match function_body {
        TokenTree::Group(group) => instrument_code(&section_label, group.stream().into_iter(), false),
        _ => panic!("expected a group for the function body but got something else"),
    };

    let mut instrumented = TokenStream::new();
    instrumented.extend(function_decl_prelude);
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
