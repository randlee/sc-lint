//! Parsing and types for `#[sc_lint(...)]` directives and site allowances,
//! shared by `sc-lint-attributes` and the `sc-lint` analyzers.
//!
//! The primary product is the [`sc-lint`](https://crates.io/crates/sc-lint) CLI;
//! install it rather than this crate unless you are building on the library.
//! Documentation and source: <https://github.com/randlee/sc-lint>.

use std::fmt;
use syn::Error;
use syn::Ident;
use syn::LitInt;
use syn::LitStr;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

/// Recognizes the supported unqualified and crate-qualified directive attributes.
pub fn is_sc_lint_attribute(attr: &syn::Attribute) -> bool {
    let parts = &attr.path().segments;
    parts.last().is_some_and(|part| part.ident == "sc_lint")
        && (parts.len() == 1 || (parts.len() == 2 && parts[0].ident == "sc_lint_attributes"))
}

/// Finds the first rule owned by an analyzer prefix but absent from its catalog.
/// Rules belonging to other analyzers are left for their owning analyzer.
pub fn unknown_owned_rule<'a>(
    rules: &'a [SiteRuleId],
    prefix: &str,
    known: &[&str],
) -> Option<&'a SiteRuleId> {
    rules
        .iter()
        .find(|rule| rule.as_str().starts_with(prefix) && !known.contains(&rule.as_str()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scope {
    Boundary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Allow(Vec<String>),
    SiteAllow {
        rules: Vec<SiteRuleId>,
        reason: SiteAllowReason,
    },
    InternalOnly,
    ForbidExternalImpls,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SiteRuleId(String);

impl SiteRuleId {
    pub fn parse(value: impl Into<String>) -> std::result::Result<Self, DirectiveError> {
        let value = value.into();
        let parts = value.split('-').collect::<Vec<_>>();
        let prefix_valid = parts.len() >= 2
            && parts[..parts.len() - 1].iter().all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
            });
        if value.trim() != value
            || !prefix_valid
            || parts
                .last()
                .is_none_or(|part| part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()))
        {
            return Err(DirectiveError::new(
                "MALFORMED_RULE_ID",
                "rule id must be an unquoted identifier such as PORT-010",
                "write the rule id as an identifier with its analyzer prefix and numeric suffix",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SiteRuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteAllowReason(String);

impl SiteAllowReason {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, DirectiveError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DirectiveError::new(
                "EMPTY_REASON",
                "site allowance reason must not be empty",
                "add a specific reason string",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirectiveError {
    code: &'static str,
    cause: String,
    recovery: String,
}

macro_rules! directive_codes {
    ($($variant:ident => ($code:literal, $recovery:literal)),+ $(,)?) => {
        /// Stable, typed categories for site allowance failures.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum DirectiveCode { $($variant),+ }
        impl DirectiveCode {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $code),+ }
            }
            pub const fn qualified(self) -> &'static str {
                match self { $(Self::$variant => concat!("DIRECTIVE.SITE_ALLOW.", $code)),+ }
            }
            pub const fn recovery(self) -> &'static str {
                match self { $(Self::$variant => $recovery),+ }
            }
            fn lookup(code: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|item| item.as_str() == code || item.qualified() == code)
            }
        }
    };
}
directive_codes! {
    EmptyRules => ("EMPTY_RULES", "provide at least one unquoted rule id"),
    MissingReason => ("MISSING_REASON", "add reason = \"...\" to the allowance"),
    EmptyReason => ("EMPTY_REASON", "provide a non-empty reason string"),
    QuotedRuleId => ("QUOTED_RULE_ID", "write rule ids as identifiers, for example PORT-010"),
    DuplicateRuleId => ("DUPLICATE_RULE_ID", "list each rule id once"),
    ReasonWithoutAllow => ("REASON_WITHOUT_ALLOW", "add an allow(...) element or remove reason"),
    MultipleAllow => ("MULTIPLE_ALLOW", "combine all rule ids in one allow(...) element"),
    DuplicateReason => ("DUPLICATE_REASON", "provide one reason element"),
    UnknownElement => ("UNKNOWN_ELEMENT", "remove unsupported elements from the attribute"),
    MalformedRuleId => ("MALFORMED_RULE_ID", "write an analyzer prefix and numeric suffix, for example PORT-010"),
    NonLiteralReason => ("NON_LITERAL_REASON", "use a string literal for the reason"),
    UnknownRule => ("UNKNOWN_RULE", "use a rule id from the owning analyzer catalog"),
    UnsupportedItem => ("UNSUPPORTED_ITEM", "place the allowance on a function or method"),
}
impl DirectiveError {
    pub fn new(code: &'static str, cause: impl Into<String>, recovery: impl Into<String>) -> Self {
        Self {
            code: DirectiveCode::lookup(code).map_or(code, DirectiveCode::qualified),
            cause: cause.into(),
            recovery: recovery.into(),
        }
    }
    pub fn typed(code: DirectiveCode, cause: impl Into<String>) -> Self {
        Self::new(code.qualified(), cause, code.recovery())
    }
    pub fn typed_code(&self) -> Option<DirectiveCode> {
        DirectiveCode::lookup(self.code)
    }
    pub fn code(&self) -> &'static str {
        self.code
    }
    pub fn cause(&self) -> &str {
        &self.cause
    }
    pub fn recovery(&self) -> &str {
        &self.recovery
    }
    /// Wraps foreign parser errors; structured errors use the typed parsing API.
    pub fn from_diagnostic(message: &str) -> Self {
        Self::typed(DirectiveCode::UnknownElement, message)
    }
}
/// Constructs an actionable error for an unknown analyzer-owned rule.
pub fn unknown_rule(rule: &SiteRuleId, known: &[&str]) -> DirectiveError {
    DirectiveError::typed(
        DirectiveCode::UnknownRule,
        format!(
            "unknown rule `{rule}`; valid rule ids: {}",
            known.join(", ")
        ),
    )
}
/// Returns every unknown rule owned by the given analyzer.
pub fn unknown_owned_rules<'a>(
    rules: &'a [SiteRuleId],
    prefix: &str,
    known: &[&str],
) -> Vec<&'a SiteRuleId> {
    rules
        .iter()
        .filter(|rule| rule.as_str().starts_with(prefix) && !known.contains(&rule.as_str()))
        .collect()
}

impl fmt::Display for DirectiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}; recovery: {}",
            self.code, self.cause, self.recovery
        )
    }
}
impl std::error::Error for DirectiveError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeInput {
    pub directives: Vec<Directive>,
}

type TypedResult<T> = std::result::Result<T, ParseFailure>;
enum ParseFailure {
    Structured(proc_macro2::Span, DirectiveError),
    Foreign(Error),
}
impl From<Error> for ParseFailure {
    fn from(error: Error) -> Self {
        Self::Foreign(error)
    }
}
impl ParseFailure {
    fn at(input: ParseStream<'_>, code: DirectiveCode, cause: &str) -> Self {
        Self::Structured(input.span(), DirectiveError::typed(code, cause))
    }
    fn into_syn(self) -> Error {
        match self {
            Self::Structured(span, error) => Error::new(span, error.to_string()),
            Self::Foreign(error) => error,
        }
    }
}
impl AttributeInput {
    /// Parses tokens while preserving typed diagnostic identity.
    pub fn parse_tokens(
        tokens: proc_macro2::TokenStream,
    ) -> std::result::Result<Self, DirectiveError> {
        use syn::parse::Parser;
        let mut diagnostic = None;
        let result = (|input: ParseStream<'_>| {
            Self::parse_typed(input).map_err(|failure| {
                if let ParseFailure::Structured(_, error) = &failure {
                    diagnostic = Some(error.clone());
                }
                failure.into_syn()
            })
        })
        .parse2(tokens);
        result.map_err(|error| {
            diagnostic.unwrap_or_else(|| DirectiveError::from_diagnostic(&error.to_string()))
        })
    }
    /// Parses a directive attribute without decoding rendered `syn` errors.
    pub fn parse_attribute(attr: &syn::Attribute) -> std::result::Result<Self, DirectiveError> {
        match &attr.meta {
            syn::Meta::List(list) => Self::parse_tokens(list.tokens.clone()),
            _ => Err(DirectiveError::typed(
                DirectiveCode::UnknownElement,
                "expected parenthesized directive arguments",
            )),
        }
    }
    fn parse_typed(input: ParseStream<'_>) -> TypedResult<Self> {
        let mut directives = Vec::new();
        let mut parts = SiteAllowParts::default();
        while !input.is_empty() {
            if input.peek(Ident) && input.peek2(Token![.]) {
                directives.push(parse_directive(input)?);
            } else {
                parts.parse_element(input)?;
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        parts.finish(input, &mut directives)?;
        Ok(Self { directives })
    }
}
impl Parse for AttributeInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Self::parse_typed(input).map_err(ParseFailure::into_syn)
    }
}
#[derive(Default)]
struct SiteAllowParts {
    rules: Option<Vec<SiteRuleId>>,
    reason: Option<SiteAllowReason>,
}
impl SiteAllowParts {
    fn parse_element(&mut self, input: ParseStream<'_>) -> TypedResult<()> {
        if !input.peek(Ident) {
            return Err(ParseFailure::at(
                input,
                DirectiveCode::UnknownElement,
                "expected a directive element",
            ));
        }
        let key = input.parse::<Ident>()?;
        match key.to_string().as_str() {
            "allow" => {
                if self.rules.is_some() {
                    return Err(ParseFailure::at(
                        input,
                        DirectiveCode::MultipleAllow,
                        "multiple site allow elements",
                    ));
                }
                self.rules = Some(parse_site_rules(input)?);
            }
            "reason" => {
                if self.reason.is_some() {
                    return Err(ParseFailure::at(
                        input,
                        DirectiveCode::DuplicateReason,
                        "duplicate site allow reason",
                    ));
                }
                input.parse::<Token![=]>()?;
                if !input.peek(LitStr) {
                    return Err(ParseFailure::at(
                        input,
                        DirectiveCode::NonLiteralReason,
                        "reason must be a string literal",
                    ));
                }
                let literal = input.parse::<LitStr>()?;
                self.reason = Some(
                    SiteAllowReason::new(literal.value())
                        .map_err(|error| ParseFailure::Structured(literal.span(), error))?,
                );
            }
            _ => {
                return Err(ParseFailure::at(
                    input,
                    DirectiveCode::UnknownElement,
                    "unknown site directive element",
                ));
            }
        }
        Ok(())
    }
    fn finish(self, input: ParseStream<'_>, directives: &mut Vec<Directive>) -> TypedResult<()> {
        match (self.rules, self.reason) {
            (Some(rules), Some(reason)) => directives.push(Directive::SiteAllow { rules, reason }),
            (Some(_), None) => {
                return Err(ParseFailure::at(
                    input,
                    DirectiveCode::MissingReason,
                    "site allow requires `reason = \"...\"`",
                ));
            }
            (None, Some(_)) => {
                return Err(ParseFailure::at(
                    input,
                    DirectiveCode::ReasonWithoutAllow,
                    "reason requires a site allow",
                ));
            }
            (None, None) => (),
        }
        Ok(())
    }
}
fn rule_group(input: ParseStream<'_>) -> Result<syn::parse::ParseBuffer<'_>> {
    let content;
    syn::parenthesized!(content in input);
    Ok(content)
}
fn parse_site_rules(input: ParseStream<'_>) -> TypedResult<Vec<SiteRuleId>> {
    let content = rule_group(input)?;
    let mut rules: Vec<SiteRuleId> = Vec::new();
    while !content.is_empty() {
        if content.peek(LitStr) {
            return Err(ParseFailure::at(
                &content,
                DirectiveCode::QuotedRuleId,
                "rule ids must be unquoted identifiers",
            ));
        }
        if !content.peek(Ident) {
            return Err(ParseFailure::at(
                &content,
                DirectiveCode::MalformedRuleId,
                "expected a rule identifier",
            ));
        }
        let mut rule = content.parse::<Ident>()?.to_string();
        loop {
            if !content.peek(Token![-]) {
                return Err(ParseFailure::at(
                    &content,
                    DirectiveCode::MalformedRuleId,
                    "expected a numeric rule suffix",
                ));
            }
            content.parse::<Token![-]>()?;
            rule.push('-');
            if content.peek(Ident) {
                rule.push_str(&content.parse::<Ident>()?.to_string());
            } else if content.peek(LitInt) {
                rule.push_str(&content.parse::<LitInt>()?.to_string());
                break;
            } else {
                return Err(ParseFailure::at(
                    &content,
                    DirectiveCode::MalformedRuleId,
                    "malformed site allow rule id",
                ));
            }
        }
        if rules.iter().any(|existing| existing.as_str() == rule) {
            return Err(ParseFailure::at(
                &content,
                DirectiveCode::DuplicateRuleId,
                "duplicate site allow rule id",
            ));
        }
        rules.push(
            SiteRuleId::parse(rule)
                .map_err(|error| ParseFailure::Structured(content.span(), error))?,
        );
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
    }
    if rules.is_empty() {
        return Err(ParseFailure::at(
            input,
            DirectiveCode::EmptyRules,
            "site allow requires at least one rule id",
        ));
    }
    Ok(rules)
}

pub(crate) fn parse_directive(input: ParseStream<'_>) -> Result<Directive> {
    let scope = parse_scope(input)?;
    input.parse::<Token![.]>()?;
    let action = input.parse::<Ident>()?;
    let action_name = action.to_string();

    match (scope, action_name.as_str()) {
        (Scope::Boundary, "allow") => {
            let content;
            syn::parenthesized!(content in input);
            let mut rule_ids = Vec::new();
            while !content.is_empty() {
                let lit = content.parse::<LitStr>()?;
                let rule_id = lit.value();
                if rule_id.trim().is_empty() {
                    return Err(Error::new(
                        lit.span(),
                        "boundary.allow rule ids must not be empty",
                    ));
                }
                rule_ids.push(rule_id);
                if content.is_empty() {
                    break;
                }
                content.parse::<Token![,]>()?;
            }
            if rule_ids.is_empty() {
                return Err(Error::new(
                    action.span(),
                    "boundary.allow requires at least one rule id string",
                ));
            }
            Ok(Directive::Allow(rule_ids))
        }
        (Scope::Boundary, "internal_only") => Ok(Directive::InternalOnly),
        (Scope::Boundary, "forbid_external_impls") => Ok(Directive::ForbidExternalImpls),
        (Scope::Boundary, _) => Err(Error::new(
            action.span(),
            format!(
                "unsupported boundary directive `{action_name}`; supported: allow(...), internal_only, forbid_external_impls"
            ),
        )),
    }
}

fn parse_scope(input: ParseStream<'_>) -> Result<Scope> {
    let ident = input.parse::<Ident>()?;
    match ident.to_string().as_str() {
        "boundary" => Ok(Scope::Boundary),
        other => Err(Error::new(
            ident.span(),
            format!("unsupported sc_lint scope `{other}`; supported: boundary"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::AttributeInput;
    use super::Directive;
    use super::SiteAllowReason;
    use super::SiteRuleId;
    use quote::quote;

    #[test]
    fn parses_boundary_allow_rule() {
        let parsed: AttributeInput =
            syn::parse2(quote!(boundary.allow("cycle.type_method_self_loop"))).unwrap();
        assert_eq!(
            parsed.directives,
            vec![Directive::Allow(vec![
                "cycle.type_method_self_loop".to_string()
            ])]
        );
    }

    #[test]
    fn site_allow_grammar_corpus() {
        let accepted = [
            (
                quote!(allow(PORT - 010), reason = "POSIX-only item"),
                vec!["PORT-010"],
            ),
            (
                quote!(
                    allow(PORT - 001, PORT - 010),
                    reason = "two deliberate exceptions"
                ),
                vec!["PORT-001", "PORT-010"],
            ),
            (
                quote!(reason = "POSIX-only item", allow(PORT - 010,),),
                vec!["PORT-010"],
            ),
            (
                quote!(allow(SCB - MANIFEST - 004), reason = "other analyzer rule"),
                vec!["SCB-MANIFEST-004"],
            ),
        ];
        for (tokens, expected_rules) in accepted {
            let parsed: AttributeInput = syn::parse2(tokens).expect("accepted site allowance");
            assert!(
                matches!(&parsed.directives[..], [Directive::SiteAllow { rules, reason }]
                if rules.iter().map(SiteRuleId::as_str).collect::<Vec<_>>() == expected_rules && !reason.as_str().is_empty())
            );
        }
        let combined: AttributeInput = syn::parse2(quote!(
            boundary.allow("B"),
            allow(PORT - 010),
            reason = "combined"
        ))
        .expect("boundary and site directives coexist");
        assert!(
            matches!(&combined.directives[..], [Directive::Allow(values), Directive::SiteAllow { .. }] if values == &["B"])
        );

        for (tokens, label, code) in [
            (
                quote!(allow("PORT-010"), reason = "quoted id"),
                "quoted rule id",
                "QUOTED_RULE_ID",
            ),
            (
                quote!(allow(PORT - 010)),
                "missing reason",
                "MISSING_REASON",
            ),
            (
                quote!(allow(), reason = "missing id"),
                "empty rules",
                "EMPTY_RULES",
            ),
            (
                quote!(allow(PORT - 010), reason = "  "),
                "empty reason",
                "EMPTY_REASON",
            ),
            (
                quote!(reason = "x"),
                "reason without allow",
                "REASON_WITHOUT_ALLOW",
            ),
            (
                quote!(allow(PORT - 010, PORT - 010), reason = "duplicate"),
                "duplicate rule",
                "DUPLICATE_RULE_ID",
            ),
            (
                quote!(allow(PORT - 010), allow(PORT - 002), reason = "multiple"),
                "multiple allows",
                "MULTIPLE_ALLOW",
            ),
            (
                quote!(allow(PORT - 010), reason = "x", reason = "y"),
                "duplicate reason",
                "DUPLICATE_REASON",
            ),
            (
                quote!(allow(PORT - 010), note = "wrong key"),
                "unknown element",
                "UNKNOWN_ELEMENT",
            ),
            (
                quote!(allow(PORT-), reason = "malformed"),
                "malformed rule id",
                "MALFORMED_RULE_ID",
            ),
            (
                quote!(allow(PORT - 010), reason = variable),
                "nonliteral reason",
                "NON_LITERAL_REASON",
            ),
        ] {
            let error = syn::parse2::<AttributeInput>(tokens).expect_err(label);
            assert!(error.to_string().contains(code), "{label}: {error}");
        }
    }

    #[test]
    fn site_allow_error_codes_are_prefixed_and_actionable() {
        let error = SiteAllowReason::new(" \t").expect_err("blank reasons are rejected");
        assert_eq!(error.code(), "DIRECTIVE.SITE_ALLOW.EMPTY_REASON");
        assert_eq!(error.cause(), "site allowance reason must not be empty");
        assert_eq!(error.recovery(), "add a specific reason string");
    }

    #[test]
    fn parses_boundary_internal_only() {
        let parsed: AttributeInput = syn::parse2(quote!(boundary.internal_only)).unwrap();
        assert_eq!(parsed.directives, vec![Directive::InternalOnly]);
    }

    #[test]
    fn parses_boundary_forbid_external_impls() {
        let parsed: AttributeInput = syn::parse2(quote!(boundary.forbid_external_impls)).unwrap();
        assert_eq!(parsed.directives, vec![Directive::ForbidExternalImpls]);
    }

    #[test]
    fn parses_multiple_directives() {
        let parsed: AttributeInput = syn::parse2(quote!(
            boundary.internal_only,
            boundary.forbid_external_impls,
            boundary.allow("cycle.type_method_self_loop")
        ))
        .unwrap();
        assert_eq!(
            parsed.directives,
            vec![
                Directive::InternalOnly,
                Directive::ForbidExternalImpls,
                Directive::Allow(vec!["cycle.type_method_self_loop".to_string()]),
            ]
        );
    }

    #[test]
    fn parses_frozen_boundary_directive_grammar_corpus() {
        let cases = [
            (
                quote!(boundary.allow("cycle.recursive_value_container")),
                vec![Directive::Allow(vec![
                    "cycle.recursive_value_container".to_string(),
                ])],
            ),
            (
                quote!(boundary.allow("cycle.type_method_self_loop")),
                vec![Directive::Allow(vec![
                    "cycle.type_method_self_loop".to_string(),
                ])],
            ),
            (
                quote!(boundary.allow(
                    "cycle.type_method_self_loop",
                    "cycle.recursive_value_container"
                )),
                vec![Directive::Allow(vec![
                    "cycle.type_method_self_loop".to_string(),
                    "cycle.recursive_value_container".to_string(),
                ])],
            ),
            (
                quote!(boundary.internal_only),
                vec![Directive::InternalOnly],
            ),
            (
                quote!(boundary.forbid_external_impls),
                vec![Directive::ForbidExternalImpls],
            ),
            (
                quote!(
                    boundary.internal_only,
                    boundary.forbid_external_impls,
                    boundary.allow("cycle.type_method_self_loop")
                ),
                vec![
                    Directive::InternalOnly,
                    Directive::ForbidExternalImpls,
                    Directive::Allow(vec!["cycle.type_method_self_loop".to_string()]),
                ],
            ),
            (
                quote!(boundary.allow("cycle.type_method_self_loop",)),
                vec![Directive::Allow(vec![
                    "cycle.type_method_self_loop".to_string(),
                ])],
            ),
            (
                quote!(boundary.allow(
                    "cycle.type_method_self_loop",
                    "cycle.recursive_value_container",
                )),
                vec![Directive::Allow(vec![
                    "cycle.type_method_self_loop".to_string(),
                    "cycle.recursive_value_container".to_string(),
                ])],
            ),
            (
                quote!(boundary.internal_only,),
                vec![Directive::InternalOnly],
            ),
            (
                quote!(boundary.allow("cycle.type_method_self_loop"),),
                vec![Directive::Allow(vec![
                    "cycle.type_method_self_loop".to_string(),
                ])],
            ),
            (
                quote!(boundary.forbid_external_impls,),
                vec![Directive::ForbidExternalImpls],
            ),
            (
                quote!(
                    boundary.internal_only,
                    boundary.forbid_external_impls,
                    boundary.allow("cycle.type_method_self_loop"),
                ),
                vec![
                    Directive::InternalOnly,
                    Directive::ForbidExternalImpls,
                    Directive::Allow(vec!["cycle.type_method_self_loop".to_string()]),
                ],
            ),
        ];

        for (input, expected) in cases {
            let parsed: AttributeInput = syn::parse2(input.clone()).unwrap();
            assert_eq!(parsed.directives, expected, "input: {input}");
        }
    }

    #[test]
    fn rejects_unknown_boundary_directive() {
        let error = syn::parse2::<AttributeInput>(quote!(boundary.unknown)).unwrap_err();
        assert!(error.to_string().contains("unsupported boundary directive"));
    }
}

#[cfg(test)]
mod site_policy_tests {
    use super::*;
    #[test]
    fn shared_attribute_paths_and_owned_rule_catalog_checks() {
        for (attribute, expected) in [
            ("sc_lint", true),
            ("sc_lint_attributes::sc_lint", true),
            ("foreign::sc_lint", false),
        ] {
            let file = syn::parse_file(&format!(
                "#[{attribute}(allow(PORT-010),reason=\"adapter\")] fn f(){{}} "
            ))
            .unwrap();
            let syn::Item::Fn(item) = &file.items[0] else {
                panic!("function")
            };
            assert_eq!(is_sc_lint_attribute(&item.attrs[0]), expected);
        }
        let rules =
            ["PORT-010", "PORT-999", "SCB-RUNTIME-007"].map(|id| SiteRuleId::parse(id).unwrap());
        assert_eq!(
            unknown_owned_rule(&rules, "PORT-", &["PORT-010"])
                .unwrap()
                .as_str(),
            "PORT-999"
        );
        assert!(unknown_owned_rule(&rules, "SCB-RUNTIME-", &["SCB-RUNTIME-007"]).is_none());
    }
}

#[cfg(test)]
mod typed_diagnostic_tests {
    use super::*;
    #[test]
    fn structured_parser_preserves_code_without_reading_cause() {
        for (tokens, code) in [
            (
                quote::quote!(allow("PORT-010"), reason = "x"),
                DirectiveCode::QuotedRuleId,
            ),
            (
                quote::quote!(allow(PORT - 010)),
                DirectiveCode::MissingReason,
            ),
            (
                quote::quote!(allow(PORT - 010), reason = some_value),
                DirectiveCode::NonLiteralReason,
            ),
        ] {
            assert_eq!(
                AttributeInput::parse_tokens(tokens)
                    .unwrap_err()
                    .typed_code(),
                Some(code)
            );
        }
        let foreign = DirectiveError::from_diagnostic("foreign text contains MISSING_REASON");
        assert_eq!(foreign.typed_code(), Some(DirectiveCode::UnknownElement));
    }
    #[test]
    fn unknown_rules_include_catalog_and_keep_all_unknown_ids() {
        let rules =
            ["PORT-998", "PORT-999", "SCB-RUNTIME-999"].map(|id| SiteRuleId::parse(id).unwrap());
        let unknown = unknown_owned_rules(&rules, "PORT-", &["PORT-010"]);
        assert_eq!(unknown.len(), 2);
        for rule in unknown {
            let error = unknown_rule(rule, &["PORT-010"]);
            assert_eq!(error.typed_code(), Some(DirectiveCode::UnknownRule));
            assert!(error.cause().contains("PORT-010"));
        }
    }
}
