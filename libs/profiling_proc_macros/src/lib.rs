use proc_macro::{ TokenTree, TokenStream, Literal };

#[cfg(feature = "profiling")]
use proc_macro::{ Group, Delimiter };
#[cfg(feature = "profiling")]
use std::{ str::FromStr, iter };

const MAX_PROFILE_SECTIONS: usize = if cfg!(feature = "profiling") { 4096 } else { 0 };

// STRETCH GOAL
//
// Figure out a way to bake initialization of the profile sections array in the global profiler at
// compile time so that we only ever have as many profile sections as we actually need. Then we
// can change the array type from [Option<ProfileSection>; MAX_PROFILE_SECTIONS] to
// [ProfileSection; PROFILE_COUNT].

#[cfg(feature = "profiling")]
// Note sure if I really want bytes_expression to be Option<TokenStream>
fn instrument_code<T>(label: &str, bytes_expression: Option<TokenStream>, code: T, include_manual_drop: bool) -> TokenStream
where T: Iterator<Item = proc_macro::TokenTree> {
    static mut PROFILE_COUNT: usize = 0;

    if label.trim_matches('"').is_empty() { panic!("empty string labels not allowed"); }

    let index = unsafe { PROFILE_COUNT };
    unsafe { PROFILE_COUNT += 1 };

    if unsafe { PROFILE_COUNT } >= MAX_PROFILE_SECTIONS {
        panic!("Exceeded max profile section count of {}", MAX_PROFILE_SECTIONS);
    }

    let autoprofile_varname = format!("__auto_profile_{index}");
    let profile_section_begin = TokenStream::from_str(format!(
        r#"let {autoprofile_varname} = performance_metrics::__AutoProfile::new({label}, {index}, {});"#,
        if bytes_expression.is_some() { bytes_expression.unwrap().to_string() } else { "0".to_string() }
    ).as_str());


    let mut instrumented_code = TokenStream::new();
    instrumented_code.extend(profile_section_begin);
    instrumented_code.extend(code);
    if include_manual_drop {
        instrumented_code.extend(TokenStream::from_str(format!("drop({autoprofile_varname});").as_str()));
    }

    instrumented_code
}

fn parse_label(literal: Literal) -> String {
    let raw_literal = literal.to_string();
    if !(raw_literal.starts_with('"') || raw_literal.starts_with("r#\"")) {
        panic!("expected string literal label but got {}", raw_literal);
    }

    raw_literal
}

#[proc_macro]
pub fn profile(input: TokenStream) -> TokenStream {
    let mut token_tree_iterator = input.into_iter().peekable();

    #[allow(unused_variables)]
    let section_label = match token_tree_iterator.next() {
        Some(TokenTree::Literal(literal)) => parse_label(literal),
        _ => panic!("Expected string literal label")
    };

    #[allow(unused_variables)]
    // TODO find a better way to express this
    let bytes_expression = if matches!(token_tree_iterator.peek(), Some(TokenTree::Group(_))) {
        // Don't like having to match since we know for sure it's a group
        match token_tree_iterator.next() {
            Some(TokenTree::Group(group)) => Some(group.stream()),
            _ => unreachable!()
        }
    } else {
        None
    };

    #[allow(unused_variables)]
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

    let code = token_tree_iterator;

    #[cfg(not(feature = "profiling"))]
    {
        let mut code_ts = TokenStream::new();
        code_ts.extend(code);
        code_ts
    }

    #[cfg(feature = "profiling")]
    instrument_code(&section_label, bytes_expression, code, include_manual_drop)
}

#[cfg(feature = "profiling")]
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
        Some(TokenTree::Literal(literal)) => parse_label(literal),
        None => format!(r#""{function_name}""#),
        _ => panic!("Expected attribute to contain a string literal"),
    };

    let bytes_expression = match attribute_token_tree.next() {
        Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {
            let mut ts = TokenStream::new();
            ts.extend(attribute_token_tree);
            Some(ts)
        },
        Some(_) => panic!("expected comma (,) then bytes_expression"),
        None => None,
    };

    // The last token in the function token stream should be a group containing the function body.
    let instrumented_function_body = match function_body {
        TokenTree::Group(group) => instrument_code(&section_label, bytes_expression, group.stream().into_iter(), false),
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

#[cfg(not(feature = "profiling"))]
#[proc_macro_attribute]
pub fn profile_function(_attribute: TokenStream, function: TokenStream) -> TokenStream { function }

#[proc_macro]
pub fn __get_max_profile_sections(_: TokenStream) -> TokenStream {
    TokenStream::from(
        TokenTree::Literal(proc_macro::Literal::usize_suffixed(MAX_PROFILE_SECTIONS))
    )
}
