use std::fmt;
use syn::Error;
use syn::Ident;
use syn::LitInt;
use syn::LitStr;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

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

impl DirectiveError {
    pub fn new(code: &'static str, cause: impl Into<String>, recovery: impl Into<String>) -> Self {
        Self {
            code: qualify_site_allow_code(code),
            cause: cause.into(),
            recovery: recovery.into(),
        }
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
    pub fn from_diagnostic(message: &str) -> Self {
        let code = [
            "EMPTY_RULES",
            "MISSING_REASON",
            "EMPTY_REASON",
            "QUOTED_RULE_ID",
            "DUPLICATE_RULE_ID",
            "REASON_WITHOUT_ALLOW",
            "MULTIPLE_ALLOW",
            "DUPLICATE_REASON",
            "UNKNOWN_ELEMENT",
            "MALFORMED_RULE_ID",
            "NON_LITERAL_REASON",
        ]
        .into_iter()
        .find(|code| message.contains(code))
        .unwrap_or_else(|| {
            if message.contains("expected identifier") {
                "QUOTED_RULE_ID"
            } else {
                "UNKNOWN_ELEMENT"
            }
        });
        let recovery = match code {
            "EMPTY_RULES" => "provide at least one unquoted rule id",
            "MISSING_REASON" => "add reason = \"...\" to the allowance",
            "EMPTY_REASON" => "provide a non-empty reason string",
            "QUOTED_RULE_ID" => "write rule ids as identifiers, for example PORT-010",
            "DUPLICATE_RULE_ID" => "list each rule id once",
            "REASON_WITHOUT_ALLOW" => "add an allow(...) element or remove reason",
            "MULTIPLE_ALLOW" => "combine all rule ids in one allow(...) element",
            "DUPLICATE_REASON" => "provide one reason element",
            "UNKNOWN_ELEMENT" => "remove unsupported elements from the attribute",
            "MALFORMED_RULE_ID" => {
                "write an analyzer prefix and numeric suffix, for example PORT-010"
            }
            "NON_LITERAL_REASON" => "use a string literal for the reason",
            _ => "correct the site allowance syntax",
        };
        let qualified_code = qualify_site_allow_code(code);
        let cause = message
            .strip_prefix(qualified_code)
            .and_then(|remainder| remainder.strip_prefix(": "))
            .and_then(|remainder| remainder.split_once("; recovery:").map(|(cause, _)| cause))
            .unwrap_or(message);
        Self::new(code, cause.to_owned(), recovery)
    }
}

fn qualify_site_allow_code(code: &'static str) -> &'static str {
    match code {
        "EMPTY_RULES" => "DIRECTIVE.SITE_ALLOW.EMPTY_RULES",
        "MISSING_REASON" => "DIRECTIVE.SITE_ALLOW.MISSING_REASON",
        "EMPTY_REASON" => "DIRECTIVE.SITE_ALLOW.EMPTY_REASON",
        "QUOTED_RULE_ID" => "DIRECTIVE.SITE_ALLOW.QUOTED_RULE_ID",
        "DUPLICATE_RULE_ID" => "DIRECTIVE.SITE_ALLOW.DUPLICATE_RULE_ID",
        "REASON_WITHOUT_ALLOW" => "DIRECTIVE.SITE_ALLOW.REASON_WITHOUT_ALLOW",
        "MULTIPLE_ALLOW" => "DIRECTIVE.SITE_ALLOW.MULTIPLE_ALLOW",
        "DUPLICATE_REASON" => "DIRECTIVE.SITE_ALLOW.DUPLICATE_REASON",
        "UNKNOWN_ELEMENT" => "DIRECTIVE.SITE_ALLOW.UNKNOWN_ELEMENT",
        "MALFORMED_RULE_ID" => "DIRECTIVE.SITE_ALLOW.MALFORMED_RULE_ID",
        "NON_LITERAL_REASON" => "DIRECTIVE.SITE_ALLOW.NON_LITERAL_REASON",
        other => other,
    }
}

fn structure_site_allow_parse_error(error: Error) -> Error {
    if error.to_string().contains("DIRECTIVE.SITE_ALLOW.") {
        return error;
    }
    let structured = DirectiveError::from_diagnostic(&error.to_string());
    Error::new(error.span(), structured.to_string())
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

impl Parse for AttributeInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut directives = Vec::new();
        let mut site_allow = SiteAllowParts::default();
        while !input.is_empty() {
            if input.peek(Ident) && !input.peek2(Token![.]) {
                site_allow.parse_element(input)?;
            } else {
                directives.push(parse_directive(input)?);
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        site_allow.finish(input, &mut directives)?;
        Ok(Self { directives })
    }
}

#[derive(Default)]
struct SiteAllowParts {
    rules: Option<Vec<SiteRuleId>>,
    reason: Option<SiteAllowReason>,
}

impl SiteAllowParts {
    fn parse_element(&mut self, input: ParseStream<'_>) -> Result<()> {
        let key = input
            .parse::<Ident>()
            .map_err(structure_site_allow_parse_error)?;
        match key.to_string().as_str() {
            "allow" => self.parse_rules(input, &key),
            "reason" => self.parse_reason(input, &key),
            _ => Err(Error::new(
                key.span(),
                site_allow_error_message(
                    "UNKNOWN_ELEMENT",
                    "unknown site directive element",
                    "remove unsupported elements from the attribute",
                ),
            )),
        }
    }

    fn parse_rules(&mut self, input: ParseStream<'_>, key: &Ident) -> Result<()> {
        if self.rules.is_some() {
            return Err(Error::new(
                key.span(),
                site_allow_error_message(
                    "MULTIPLE_ALLOW",
                    "multiple site allow elements",
                    "combine all rule ids in one allow(...) element",
                ),
            ));
        }
        self.rules = Some(parse_site_rules(input, key)?);
        Ok(())
    }

    fn parse_reason(&mut self, input: ParseStream<'_>, key: &Ident) -> Result<()> {
        if self.reason.is_some() {
            return Err(Error::new(
                key.span(),
                site_allow_error_message(
                    "DUPLICATE_REASON",
                    "duplicate site allow reason",
                    "provide one reason element",
                ),
            ));
        }
        input
            .parse::<Token![=]>()
            .map_err(structure_site_allow_parse_error)?;
        if !input.peek(LitStr) {
            return Err(Error::new(
                key.span(),
                site_allow_error_message(
                    "NON_LITERAL_REASON",
                    "reason must be a string literal",
                    "use a string literal for the reason",
                ),
            ));
        }
        let literal = input.parse::<LitStr>()?;
        self.reason = Some(
            SiteAllowReason::new(literal.value())
                .map_err(|error| Error::new(literal.span(), error.to_string()))?,
        );
        Ok(())
    }

    fn finish(self, input: ParseStream<'_>, directives: &mut Vec<Directive>) -> Result<()> {
        match (self.rules, self.reason) {
            (Some(rules), Some(reason)) => {
                directives.push(Directive::SiteAllow { rules, reason });
                Ok(())
            }
            (Some(_), None) => Err(input.error(site_allow_error_message(
                "MISSING_REASON",
                "site allow requires `reason = \"...\"`",
                "add reason = \"...\" to the allowance",
            ))),
            (None, Some(_)) => Err(input.error(site_allow_error_message(
                "REASON_WITHOUT_ALLOW",
                "reason requires a site allow",
                "add an allow(...) element or remove reason",
            ))),
            (None, None) => Ok(()),
        }
    }
}

fn site_allow_error_message(code: &'static str, cause: &str, recovery: &str) -> String {
    DirectiveError::new(code, cause, recovery).to_string()
}

fn parse_site_rules(input: ParseStream<'_>, allow_ident: &Ident) -> Result<Vec<SiteRuleId>> {
    parse_site_rules_inner(input, allow_ident).map_err(structure_site_allow_parse_error)
}

fn parse_site_rules_inner(input: ParseStream<'_>, allow_ident: &Ident) -> Result<Vec<SiteRuleId>> {
    let content;
    syn::parenthesized!(content in input);
    let mut rules: Vec<SiteRuleId> = Vec::new();
    while !content.is_empty() {
        if content.peek(LitStr) {
            return Err(Error::new(
                content.span(),
                DirectiveError::new(
                    "QUOTED_RULE_ID",
                    "rule ids must be unquoted identifiers",
                    "write rule ids as identifiers, for example PORT-010",
                )
                .to_string(),
            ));
        }
        let first = content.parse::<Ident>()?;
        let mut rule = first.to_string();
        loop {
            content.parse::<Token![-]>()?;
            if content.peek(Ident) {
                rule.push('-');
                rule.push_str(&content.parse::<Ident>()?.to_string());
            } else if content.peek(LitInt) {
                rule.push('-');
                rule.push_str(&content.parse::<LitInt>()?.to_string());
                break;
            } else {
                return Err(Error::new(
                    content.span(),
                    DirectiveError::new(
                        "MALFORMED_RULE_ID",
                        "malformed site allow rule id",
                        "write an analyzer prefix and numeric suffix, for example PORT-010",
                    )
                    .to_string(),
                ));
            }
        }
        if rules.iter().any(|existing| existing.as_str() == rule) {
            return Err(Error::new(
                allow_ident.span(),
                DirectiveError::new(
                    "DUPLICATE_RULE_ID",
                    "duplicate site allow rule id",
                    "list each rule id once",
                )
                .to_string(),
            ));
        }
        rules.push(
            SiteRuleId::parse(rule).map_err(|e| Error::new(allow_ident.span(), e.to_string()))?,
        );
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
    }
    if rules.is_empty() {
        return Err(Error::new(
            allow_ident.span(),
            DirectiveError::new(
                "EMPTY_RULES",
                "site allow requires at least one rule id",
                "provide at least one unquoted rule id",
            )
            .to_string(),
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
