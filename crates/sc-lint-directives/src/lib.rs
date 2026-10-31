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
    SiteAllow { rules: Vec<String>, reason: String },
    InternalOnly,
    ForbidExternalImpls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeInput {
    pub directives: Vec<Directive>,
}

impl Parse for AttributeInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut directives = Vec::new();
        let mut site_rules: Option<Vec<String>> = None;
        let mut site_reason: Option<String> = None;
        while !input.is_empty() {
            if input.peek(Ident) && !input.peek2(Token![.]) {
                let key = input.parse::<Ident>()?;
                match key.to_string().as_str() {
                    "allow" => {
                        if site_rules.is_some() {
                            return Err(Error::new(key.span(), "multiple site allow elements"));
                        }
                        site_rules = Some(parse_site_rules(input, &key)?);
                    }
                    "reason" => {
                        if site_reason.is_some() {
                            return Err(Error::new(key.span(), "duplicate site allow reason"));
                        }
                        input.parse::<Token![=]>()?;
                        let literal = input.parse::<LitStr>()?;
                        let reason = literal.value();
                        if reason.trim().is_empty() {
                            return Err(Error::new(
                                literal.span(),
                                "site allow reason must not be empty",
                            ));
                        }
                        site_reason = Some(reason);
                    }
                    _ => return Err(Error::new(key.span(), "unknown site directive element")),
                }
            } else {
                directives.push(parse_directive(input)?);
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        if let Some(rules) = site_rules {
            let reason =
                site_reason.ok_or_else(|| input.error("site allow requires `reason = \"...\"`"))?;
            directives.push(Directive::SiteAllow { rules, reason });
        } else if site_reason.is_some() {
            return Err(input.error("reason requires a site allow"));
        }
        Ok(Self { directives })
    }
}

fn parse_site_rules(input: ParseStream<'_>, allow_ident: &Ident) -> Result<Vec<String>> {
    let content;
    syn::parenthesized!(content in input);
    let mut rules = Vec::new();
    while !content.is_empty() {
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
                return Err(content.error("malformed site allow rule id"));
            }
        }
        if rules.contains(&rule) {
            return Err(Error::new(
                allow_ident.span(),
                "duplicate site allow rule id",
            ));
        }
        rules.push(rule);
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
    }
    if rules.is_empty() {
        return Err(Error::new(
            allow_ident.span(),
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
                if rules == &expected_rules.iter().map(|rule| rule.to_string()).collect::<Vec<_>>() && !reason.is_empty())
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

        for (tokens, label) in [
            (
                quote!(allow("PORT-010"), reason = "quoted id"),
                "quoted rule id",
            ),
            (quote!(allow(PORT - 010)), "missing reason"),
            (quote!(allow(), reason = "missing id"), "empty rules"),
            (quote!(allow(PORT - 010), reason = "  "), "empty reason"),
            (quote!(reason = "x"), "reason without allow"),
            (
                quote!(allow(PORT - 010, PORT - 010), reason = "duplicate"),
                "duplicate rule",
            ),
            (
                quote!(allow(PORT - 010), allow(PORT - 002), reason = "multiple"),
                "multiple allows",
            ),
            (
                quote!(allow(PORT - 010), reason = "x", reason = "y"),
                "duplicate reason",
            ),
            (
                quote!(allow(PORT - 010), note = "wrong key"),
                "unknown element",
            ),
            (
                quote!(allow(PORT-), reason = "malformed"),
                "malformed rule id",
            ),
            (
                quote!(allow(PORT - 010), reason = variable),
                "nonliteral reason",
            ),
        ] {
            assert!(syn::parse2::<AttributeInput>(tokens).is_err(), "{label}");
        }
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
