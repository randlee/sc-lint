use super::*;

const TOKIO: &str = "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }";
fn report(source: &str, tokio: bool) -> FindingsReport {
    let fixture = tokio_fixture(source, if tokio { TOKIO } else { "" });
    analyze_workspace(&analyze_options(&fixture)).unwrap()
}

#[test]
fn each_new_rule_has_positive_and_negative_source_fixtures() {
    for (rule, positive, negative, tokio) in [
        (
            RuleId::ScbRuntime004,
            "fn f(){ std::thread::sleep_until(deadline); }",
            "fn f(){ std::thread::yield_now(); }",
            true,
        ),
        (
            RuleId::ScbRuntime005,
            "async fn f(){ tokio::time::sleep(duration).await; }",
            "async fn f(){ tokio::task::yield_now().await; }",
            false,
        ),
        (
            RuleId::ScbRuntime006,
            "fn f(rt: &tokio::runtime::Runtime){ rt.block_on(future); }",
            "fn f(rt: &Custom){ rt.block_on(future); }",
            false,
        ),
        (
            RuleId::ScbRuntime007,
            "async fn f(m: &std::sync::Mutex<()>){ let guard = m.lock().unwrap(); future.await; }",
            "async fn f(m: &std::sync::Mutex<()>){ let guard = m.lock().unwrap(); drop(guard); future.await; }",
            false,
        ),
    ] {
        let found = report(positive, tokio);
        assert_eq!(rule_ids(&found), vec![rule], "{positive}");
        assert_eq!(found.status, ReportStatus::Fail);
        assert!(report(negative, tokio).findings.is_empty(), "{negative}");
    }
}

#[test]
fn sleep_until_matches_existing_tokio_package_scope() {
    let source = "fn f(){std::thread::sleep(());std::thread::sleep_until(());}";
    assert!(report(source, false).findings.is_empty());
    assert_eq!(
        rule_ids(&report(source, true)),
        vec![RuleId::ScbRuntime003, RuleId::ScbRuntime004]
    );
}

#[test]
fn supported_call_aliases_and_executor_entry_points() {
    for source in [
        "use std::thread::sleep_until as delay; fn f(){delay(());}",
        "use std::thread as thread; fn f(){thread::sleep_until(());}",
        "use std::thread::*; fn f(){sleep_until(());}",
        "use tokio::time::{self as clock}; fn f(){clock::sleep(());}",
        "use tokio::time::{sleep_until as wait}; fn f(){wait(());}",
        "use tokio as tk; use tk::time as clock; fn f(){clock::sleep_until(());}",
        "use tokio::time::*; fn f(){sleep(());}",
        "use futures::executor::block_on as run; fn f(){run(async{});}",
        "use futures::executor as exec; fn f(){exec::block_on(async{});}",
        "use tokio::runtime::Runtime as Rt; fn f(){Rt::block_on(rt, async{});}",
        "use tokio::runtime::Handle as H; fn f(rt: &H){rt.block_on(async{});}",
        "fn f(){tokio::runtime::Handle::block_on(handle, async{});}",
        "use tokio::runtime::Runtime as Rt; fn f(){let rt=Rt::new().unwrap();rt.block_on(async{});}",
        "fn f(){let rt=tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();rt.block_on(async{});}",
        "fn f(){let handle=tokio::runtime::Handle::current();handle.block_on(async{});}",
        "struct State { runtime: tokio::runtime::Runtime } impl State { fn f(&self){ self.runtime.block_on(async{}); } }",
    ] {
        assert_eq!(report(source, true).findings.len(), 1, "{source}");
    }
}

#[test]
fn tests_are_exempt_in_attributes_modules_files_and_methods() {
    let calls =
        "std::thread::sleep_until(());tokio::time::sleep(());futures::executor::block_on(async{});";
    for source in [
        format!("#[test] fn f(){{ {calls} }}"),
        format!("#[tokio::test] async fn f(){{ {calls} }}"),
        format!("#[cfg(test)] mod unit {{ fn f(){{ {calls} }} }}"),
        format!("#[cfg(all(test, unix))] fn f(){{ {calls} }}"),
        format!("#![cfg(test)] fn f(){{ {calls} }}"),
        format!("struct T; impl T {{ #[test] fn f(){{ {calls} }} }}"),
        format!("#[cfg(test)] const X: () = {{ {calls} }};"),
        format!("mod tests {{ fn f(){{ {calls} }} }}"),
        "#[test] async fn f(m:&std::sync::Mutex<()>){let guard=m.lock().unwrap();future.await;}"
            .into(),
    ] {
        assert!(report(&source, true).findings.is_empty(), "{source}");
    }
    let fixture = tokio_fixture("pub fn valid(){}", TOKIO);
    fixture.write(
        "crates/example/tests/integration.rs",
        &format!("fn f(){{ {calls} }}"),
    );
    assert!(
        analyze_workspace(&analyze_options(&fixture))
            .unwrap()
            .findings
            .is_empty()
    );
}

#[test]
fn out_of_line_test_module_marker_exempts_all_new_rules() {
    let source = r#"
fn timer() { std::thread::sleep_until(deadline); }
async fn task(m: &std::sync::Mutex<()>) {
    tokio::time::sleep(duration).await;
    futures::executor::block_on(async {});
    let guard = m.lock().unwrap();
    future.await;
}
"#;
    let fixture = tokio_fixture("#[cfg(test)] mod checks;", TOKIO);
    fixture.write("crates/example/src/checks.rs", source);
    assert_eq!(
        rule_ids(&analyze_workspace(&analyze_options(&fixture)).unwrap()),
        vec![
            RuleId::ScbRuntime004,
            RuleId::ScbRuntime005,
            RuleId::ScbRuntime006,
            RuleId::ScbRuntime007,
        ]
    );
    fixture.write(
        "crates/example/src/checks.rs",
        &format!("#![cfg(test)]\n{source}"),
    );
    assert!(
        analyze_workspace(&analyze_options(&fixture))
            .unwrap()
            .findings
            .is_empty()
    );
}

#[test]
fn reasoned_local_allows_cover_each_rule_and_do_not_leak_to_other_rules() {
    for (rule, body) in [
        ("SCB-RUNTIME-003", "std::thread::sleep(());"),
        ("SCB-RUNTIME-004", "std::thread::sleep_until(());"),
        ("SCB-RUNTIME-005", "tokio::time::sleep(());"),
        ("SCB-RUNTIME-006", "futures::executor::block_on(async{});"),
        (
            "SCB-RUNTIME-007",
            "let guard=m.lock().unwrap();future.await;",
        ),
    ] {
        let source = format!(
            "#[sc_lint(allow({rule}), reason=\"Controlled compatibility adapter\")] async fn f(m:&std::sync::Mutex<()>){{ {body} }}"
        );
        assert!(report(&source, true).findings.is_empty(), "{source}");
    }
    let source = "#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-005), reason=\"Protocol timer\")] fn f(){ tokio::time::sleep(()); futures::executor::block_on(async{}); }";
    assert_eq!(rule_ids(&report(source, true)), vec![RuleId::ScbRuntime006]);
    let nested = "#[sc_lint(allow(SCB-RUNTIME-005), reason=\"Outer only\")] fn f(){fn inner(){tokio::time::sleep(());}tokio::time::sleep(());}";
    assert_eq!(rule_ids(&report(nested, true)), vec![RuleId::ScbRuntime005]);
}

#[test]
fn missing_empty_and_unknown_allow_reasons_are_reported() {
    for (attribute, code) in [
        (
            "allow(SCB-RUNTIME-005)",
            sc_lint_directives::DirectiveCode::MissingReason.qualified(),
        ),
        (
            "allow(SCB-RUNTIME-005), reason=\"  \"",
            "DIRECTIVE.SITE_ALLOW.EMPTY_REASON",
        ),
        (
            "allow(SCB-RUNTIME-999), reason=\"unknown\"",
            sc_lint_directives::DirectiveCode::UnknownRule.qualified(),
        ),
    ] {
        let fixture = tokio_fixture(
            &format!("#[sc_lint({attribute})] fn f(){{tokio::time::sleep(());}}"),
            TOKIO,
        );
        let error = analyze_workspace(&analyze_options(&fixture))
            .unwrap_err()
            .to_string();
        assert!(error.contains(code), "{error}");
        assert!(error.contains("recovery:"), "{error}");
    }
}

#[test]
fn mutex_guards_follow_lexical_scopes_drop_aliases_and_temporary_lifetimes() {
    for body in [
        "{let guard=m.lock().unwrap(); consume(&guard);} future.await;",
        "let guard=m.lock().unwrap(); if condition {drop(guard)} else {drop(guard)} future.await;",
        "let guard=m.lock().unwrap(); match condition {true=>drop(guard),false=>drop(guard)} future.await;",
        "m.lock().unwrap(); future.await;",
        "let value=*m.lock().unwrap(); future.await;",
        "let value=m.lock().unwrap().clone(); future.await;",
        "let guard=m.lock().unwrap(); std::mem::drop(guard); future.await;",
        "use std::mem::drop as release; let guard=m.lock().unwrap(); release(guard); future.await;",
        "let future=async {let guard=m.lock().unwrap(); drop(guard); future.await;}; future.await;",
    ] {
        let source = format!("async fn f(m: &std::sync::Mutex<()>){{ {body} }}");
        assert!(report(&source, false).findings.is_empty(), "{source}");
    }
    for body in [
        "let guard=m.lock().unwrap(); future.await;",
        "let guard; guard=m.lock().unwrap(); future.await;",
        "let guard=m.try_lock().expect(\"guard\"); future.await;",
        "let guard=std::sync::Mutex::lock(m)?; future.await;",
        "let guard=m.lock().unwrap(); let guard=(); drop(guard); future.await;",
        "let guard=m.lock().unwrap(); {drop(&guard);} future.await;",
        "let guard=m.lock().unwrap(); if condition { drop(guard); } future.await;",
    ] {
        let source = format!("async fn f(m:&std::sync::Mutex<()>){{ {body} }}");
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
    }
}

#[test]
fn mutex_types_aliases_local_values_async_blocks_and_fields_are_recognized() {
    for source in [
        "use std::sync::{Arc,Mutex as Lock}; fn f(){let m=Arc::new(Lock::new(()));let future=async {let guard=m.lock().unwrap();future.await;};}",
        "async fn f(){let m=std::sync::Mutex::new(());let guard=m.lock().unwrap();future.await;}",
        "async fn f(guard:std::sync::MutexGuard<'_,()>){future.await;}",
        "struct State{mutex:std::sync::Mutex<()>} impl State{async fn f(&self){let guard=self.mutex.lock().unwrap();future.await;}}",
        "async fn f(m:&Custom){let guard:std::sync::MutexGuard<'_, ()>=m.acquire();future.await;}",
    ] {
        assert_eq!(
            rule_ids(&report(source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
    }
    assert!(
        report(
            "async fn f(m:&tokio::sync::Mutex<()>){let guard=m.lock().await;future.await;}",
            false
        )
        .findings
        .is_empty()
    );
    assert!(
        report(
            "async fn f(m:&Custom){let guard=m.lock();future.await;}",
            false
        )
        .findings
        .is_empty()
    );
}

#[test]
fn new_rules_honor_runtime_configuration_filters() {
    let fixture = tokio_fixture(
        "fn f(){tokio::time::sleep(());futures::executor::block_on(async{});}",
        "",
    );
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.runtime]\nexclude_rules=['SCB-RUNTIME-005']\nfail_rules=[]\n",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_eq!(rule_ids(&report), vec![RuleId::ScbRuntime006]);
    assert_eq!(report.status, ReportStatus::Pass);
}

#[test]
fn glob_resolution_does_not_recurse_through_self_aliases() {
    assert!(
        report("use self as here; use here::*; fn f(){ unknown(); }", false)
            .findings
            .is_empty()
    );
}

#[test]
fn guard_join_excludes_diverging_if_and_match_arms() {
    for exit in [
        "return Err(());",
        "panic!()",
        "std::panic!()",
        "unreachable!()",
        "todo!()",
        "unimplemented!()",
        "stop()",
        "std::process::exit(1)",
        "loop {}",
    ] {
        let source = format!(
            "fn stop()->!{{loop{{}}}} async fn f(m:&std::sync::Mutex<Vec<u8>>){{let g=m.lock().unwrap();let v=if let Some(v)=g.first(){{let v=*v;drop(g);v}}else{{{exit}}};other(v).await;}}"
        );
        assert!(report(&source, false).findings.is_empty(), "{source}");
        let source = format!(
            "fn stop()->!{{loop{{}}}} async fn f(m:&std::sync::Mutex<Vec<u8>>){{let g=m.lock().unwrap();let v=match g.first(){{Some(v)=>{{let v=*v;drop(g);v}},None=>{{{exit}}}}};other(v).await;}}"
        );
        assert!(report(&source, false).findings.is_empty(), "{source}");
    }
    for exit in ["break;", "continue;"] {
        let source = format!(
            "async fn f(m:&std::sync::Mutex<()>){{for _ in 0..2{{let g=m.lock().unwrap();let v=if condition{{drop(g);1}}else{{{exit}}};other(v).await;}}}}"
        );
        assert!(report(&source, false).findings.is_empty(), "{source}");
    }
}

#[test]
fn guard_join_keeps_reachable_live_branches_and_respects_never_callable_types() {
    for source in [
        "async fn f(m:&std::sync::Mutex<()>, stop:fn()->!){let g=m.lock().unwrap();let v=if condition{drop(g);1}else{stop()};other(v).await;}",
        "async fn f(m:&std::sync::Mutex<()>){let stop=|| ->! {panic!()};let g=m.lock().unwrap();let v=if condition{drop(g);1}else{stop()};other(v).await;}",
    ] {
        assert!(report(source, false).findings.is_empty(), "{source}");
    }
    for source in [
        "async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap();let v=if condition{drop(g);1}else{2};other(v).await;}",
        "fn stop()->!{panic!()} async fn f(m:&std::sync::Mutex<()>, stop:fn()->u8){let g=m.lock().unwrap();let v=if condition{drop(g);1}else{stop()};other(v).await;}",
        "async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap();let v=match condition{true=>{drop(g);1},false=>2};other(v).await;}",
    ] {
        assert_eq!(
            rule_ids(&report(source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
    }
}

#[test]
fn common_mutex_receiver_and_poison_recovery_shapes_are_checked() {
    for source in [
        "use std::sync::{Arc,Mutex};async fn f(m:Arc<Mutex<()>>){let m2=Arc::clone(&m);let g=m2.lock().unwrap();tick().await;}",
        "use std::rc::Rc;use std::sync::Mutex;async fn f(m:Rc<Mutex<()>>){let m2=Rc::clone(&m);let g=m2.lock().unwrap();tick().await;}",
        "static STATE:std::sync::Mutex<()>=std::sync::Mutex::new(());async fn f(){let g=STATE.lock().unwrap();work().await;}",
        "async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap_or_else(|e|e.into_inner());tick().await;}",
        "use std::sync::*;async fn f(){let m=Mutex::new(());let g=m.lock().unwrap();tick().await;}",
        "async fn f(m:&std::sync::Mutex<()>){if let Ok(g)=m.lock(){tick().await;}}",
    ] {
        assert_eq!(
            rule_ids(&report(source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
    }
    assert_eq!(
        rule_ids(&report(
            "use tokio::runtime::*;fn f(){let rt=Runtime::new().unwrap();rt.block_on(async{});}",
            false
        )),
        vec![RuleId::ScbRuntime006]
    );
    for source in [
        "static STATE:std::sync::Mutex<()>=std::sync::Mutex::new(());async fn f(){let STATE=Custom;let g=STATE.lock();tick().await;}",
        "async fn f(m:&std::sync::Mutex<()>){if let Ok(g)=m.lock(){drop(g);tick().await;}}",
        "async fn f(m:&std::sync::Mutex<()>){if let Ok(g)=m.lock(){consume(&g);}tick().await;}",
        "async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap_or_else(|e|e.into_inner());drop(g);tick().await;}",
    ] {
        assert!(report(source, false).findings.is_empty(), "{source}");
    }
}

#[test]
fn allowance_errors_are_all_located_source_ordered_and_not_duplicated() {
    let fixture = tokio_fixture(
        "#[sc_lint(allow(SCB-RUNTIME-999),reason=\"unknown\")]\nfn first(){}\n#[sc_lint(allow(SCB-RUNTIME-005))]\nfn second(){}",
        "",
    );
    let error = analyze_workspace(&analyze_options(&fixture))
        .unwrap_err()
        .to_string();
    assert_eq!(
        error
            .matches(sc_lint_directives::DirectiveCode::UnknownRule.qualified())
            .count(),
        1,
        "{error}"
    );
    assert_eq!(
        error
            .matches(sc_lint_directives::DirectiveCode::MissingReason.qualified())
            .count(),
        1,
        "{error}"
    );
    assert!(error.contains("UNKNOWN_RULE at line 1"), "{error}");
    assert!(error.contains("MISSING_REASON at line 3"), "{error}");
    assert!(
        error.find("UNKNOWN_RULE").unwrap() < error.find("MISSING_REASON").unwrap(),
        "{error}"
    );
}

#[test]
fn documented_conservative_move_and_loop_exit_cases_still_report() {
    // Known limitation: interprocedural moves and loop-exit ownership transfer
    // remain conservative. See README and docs-bundle syntactic coverage limits.
    for body in [
        "consume(g);other().await;",
        "loop{drop(g);break;}other().await;",
    ] {
        let source =
            format!("async fn f(m:&std::sync::Mutex<()>){{let g=m.lock().unwrap();{body}}}");
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007]
        );
    }
}

#[test]
fn unrelated_or_ambiguous_never_declarations_do_not_hide_live_guards() {
    for source in [
        "fn stop()->!{panic!()} async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap();let v=if condition{drop(g);1}else{foreign::stop()};other(v).await;}",
        "mod unrelated{fn stop()->!{panic!()}} fn stop()->u8{1} async fn f(m:&std::sync::Mutex<()>){let g=m.lock().unwrap();let v=if condition{drop(g);1}else{stop()};other(v).await;}",
    ] {
        assert_eq!(
            rule_ids(&report(source, false)),
            vec![RuleId::ScbRuntime007]
        );
    }
}

#[test]
fn known_never_associated_and_self_methods_do_not_contribute_to_joins() {
    for exit in ["self.stop()", "Self::stop(self)", "S::stop(self)"] {
        let source = format!(
            "struct S;impl S{{fn stop(&self)->!{{panic!()}}async fn f(&self,m:&std::sync::Mutex<()>){{let g=m.lock().unwrap();let v=if condition{{drop(g);1}}else{{{exit}}};other(v).await;}}}}"
        );
        for source in [source.clone(), format!("mod nested {{{source}}}")] {
            assert!(report(&source, false).findings.is_empty(), "{source}");
        }
    }
}

#[test]
fn local_label_breaks_do_not_hide_a_reachable_live_guard() {
    for tail in [
        "'inner:{break 'inner 2;}",
        "'inner:{if condition2 {break 'inner 2;}drop(g);1}",
    ] {
        let source = format!(
            "async fn f(m:&std::sync::Mutex<()>){{let g=m.lock().unwrap();let v=if condition{{drop(g);1}}else{{{tail}}};other(v).await;}}"
        );
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007]
        );
    }
}

#[test]
fn lock_result_patterns_keep_guards_live_only_in_their_scope() {
    for body in [
        "let Ok(g) = m.lock() else { return }; tick().await;",
        "match m.lock() { Ok(g) => { tick().await; }, Err(_) => {} }",
        "while let Ok(g) = m.lock() { tick().await; break; }",
    ] {
        let source = format!("async fn f(m: &std::sync::Mutex<()>) {{ {body} }}");
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{body}"
        );
    }
    for body in [
        "let Ok(g) = m.lock() else { return }; drop(g); tick().await;",
        "match m.lock() { Ok(g) => { drop(g); tick().await; }, Err(_) => {} }",
        "match m.lock() { Ok(g) => {}, Err(_) => {} }; tick().await;",
        "while let Ok(g) = m.lock() { drop(g); tick().await; break; }",
        "while let Ok(g) = m.lock() { break; } tick().await;",
        "let Ok(g) = other() else { return }; tick().await;",
    ] {
        let source = format!("async fn f(m: &std::sync::Mutex<()>) {{ {body} }}");
        assert!(report(&source, false).findings.is_empty(), "{body}");
    }
}

#[test]
fn macro_break_inside_loop_preserves_reachable_guard_state() {
    for body in [
        "if ready() { drop(g); } else { loop { tokio::select! { _ = event() => break, } } } tick().await;",
        "match ready() { true => drop(g), false => loop { tokio::select! { _ = event() => { break; } } } } tick().await;",
    ] {
        let source =
            format!("async fn f(m: &std::sync::Mutex<()>) {{ let g = m.lock().unwrap(); {body} }}");
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{body}"
        );
    }
}

#[test]
fn shared_resolver_supports_sleep_import_forms_and_preserves_name_hint_policy() {
    for (imports, expected) in [
        (
            "use std::thread::{self};",
            vec![RuleId::ScbRuntime003, RuleId::ScbRuntime004],
        ),
        (
            "use std::thread::{self as thread};",
            vec![RuleId::ScbRuntime003, RuleId::ScbRuntime004],
        ),
        (
            "use std::*;",
            vec![RuleId::ScbRuntime003, RuleId::ScbRuntime004],
        ),
    ] {
        let source = format!("{imports} fn f() {{ thread::sleep(()); thread::sleep_until(()); }}");
        assert_eq!(rule_ids(&report(&source, true)), expected, "{imports}");
    }
    let source = "mod tests { fn f() { std::thread::sleep(()); std::thread::sleep_until(()); } }";
    assert_eq!(rule_ids(&report(source, true)), vec![RuleId::ScbRuntime003]);
}

#[test]
fn lock_match_results_and_scrutinees_keep_resources_across_await() {
    for body in [
        "let g = match m.lock() { Ok(g) => g, Err(p) => p.into_inner() }; tick().await;",
        "match m.lock() { Ok(g) => {}, Err(p) => { tick().await; } }",
        "match m.lock() { Ok(_) => { tick().await; }, _ => {} }",
        "match m.lock() { Ok(g) if ready() => {}, _ => { tick().await; } }",
        "match m.lock() { Ok(_) => {}, Err(p) => { let g = p.into_inner(); tick().await; } }",
    ] {
        let source = format!("async fn f(m: &std::sync::Mutex<()>) {{ {body} }}");
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{body}"
        );
    }
    for body in [
        "let g = match m.lock() { Ok(g) => g, Err(p) => p.into_inner() }; drop(g); tick().await;",
        "match m.lock() { Ok(g) => {}, Err(p) => { drop(p); tick().await; } }",
        "drop(match m.lock() { Ok(_) => {}, _ => {} }); tick().await;",
        "drop(match m.lock() { Ok(g) if ready() => g, Ok(g) => g, Err(p) => p.into_inner() }); tick().await;",
        "match m.lock() { Ok(_) => {}, Err(p) => { drop(p.into_inner()); tick().await; } }",
        "match m.lock() { Ok(_) => {}, Err(p) => { let g = p.into_inner(); drop(g); tick().await; } }",
        "match other.lock() { Ok(_) => { tick().await; }, _ => {} }",
    ] {
        let source = format!("async fn f(m: &std::sync::Mutex<()>) {{ {body} }}");
        assert!(report(&source, false).findings.is_empty(), "{body}");
    }
}

#[test]
fn try_lock_variant_arms_track_only_recognized_guard_owners() {
    // Known limitation: unrefined try_lock errors and wildcard fallbacks are
    // not modeled as owning a guard. See README and docs-bundle limits.
    for scrutinee in ["m.try_lock()", "std::sync::Mutex::try_lock(m)"] {
        for arms in [
            "Ok(g) => { use_it(g); }, Err(_) => { tick().await; }",
            "Ok(g) => { use_it(g); }, Err(TryLockError::WouldBlock) => { tick().await; }, _ => {}",
            "Ok(g) => { use_it(g); }, Err(p) => { tick().await; }",
            "Ok(g) => { use_it(g); }, _ => { tick().await; }",
            "Ok(g) if ready() => { use_it(g); }, _ => { tick().await; }",
            "Ok(g) if ready() => { use_it(g); }, Err(p) => { tick().await; }, _ => {}",
        ] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {arms} }} }}"
            );
            assert!(report(&source, false).findings.is_empty(), "{source}");
        }
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ Ok(g) => {{ tick().await; }}, _ => {{}} }} }}"
        );
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ Err(TryLockError::Poisoned(p)) => {{ tick().await; }}, _ => {{}} }} }}"
        );
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ Err(TryLockError::Poisoned(p)) => {{ drop(p); tick().await; }}, _ => {{}} }} }}"
        );
        assert!(report(&source, false).findings.is_empty(), "{source}");
    }
}

#[test]
fn moved_match_scrutinees_are_released_by_their_pattern_name() {
    for scrutinee in ["m.lock().unwrap()", "m.lock()"] {
        for pattern in ["g", "g @ _", "(g)"] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ drop(g); tick().await; }} }} }}"
            );
            assert!(report(&source, false).findings.is_empty(), "{source}");
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ tick().await; }} }} }}"
            );
            assert_eq!(
                rule_ids(&report(&source, false)),
                vec![RuleId::ScbRuntime007],
                "{source}"
            );
        }
    }
}

#[test]
fn typed_match_move_bindings_keep_and_release_their_named_owner() {
    // Typed patterns are an AST form; construct one explicitly because Rust's
    // source grammar does not accept type ascription in match-arm patterns.
    for release in ["", "drop(g);"] {
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match m.lock() {{ g => {{ {release} tick().await; }} }} }}"
        );
        let mut file = syn::parse_file(&source).unwrap();
        let syn::Item::Fn(function) = &mut file.items[0] else {
            panic!("function fixture")
        };
        let syn::Stmt::Expr(syn::Expr::Match(branch), _) = &mut function.block.stmts[0] else {
            panic!("match fixture")
        };
        branch.arms[0].pat = syn::Pat::Type(syn::PatType {
            attrs: Vec::new(),
            pat: Box::new(branch.arms[0].pat.clone()),
            colon_token: Default::default(),
            ty: Box::new(syn::parse_str("_").unwrap()),
        });
        assert_eq!(
            crate::practices::find(&file).len(),
            usize::from(release.is_empty()),
            "{source}"
        );
    }
}

#[test]
fn dropping_a_match_reference_binding_does_not_release_its_scrutinee() {
    for scrutinee in ["m.lock()", "m.lock().unwrap()"] {
        for pattern in ["ref g", "ref g @ _"] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ drop(g); tick().await; }} }} }}"
            );
            assert_eq!(
                rule_ids(&report(&source, false)),
                vec![RuleId::ScbRuntime007],
                "{source}"
            );
        }
    }
}

#[test]
fn try_lock_ok_wildcard_and_rest_patterns_keep_the_scrutinee_guard() {
    for scrutinee in ["m.try_lock()", "std::sync::Mutex::try_lock(m)"] {
        for pattern in ["Ok(_)", "Ok(..)"] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ tick().await; }}, _ => {{}} }} }}"
            );
            assert_eq!(
                rule_ids(&report(&source, false)),
                vec![RuleId::ScbRuntime007],
                "{source}"
            );
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{}}, _ => {{}} }}; tick().await; }}"
            );
            assert!(report(&source, false).findings.is_empty(), "{source}");
        }
        for pattern in ["Err(_)", "Err(..)", "_"] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ Ok(g) => {{ use_it(g); }}, {pattern} => {{ tick().await; }} }} }}"
            );
            assert!(report(&source, false).findings.is_empty(), "{source}");
        }
    }
}

#[test]
fn lock_rest_and_or_patterns_keep_the_scrutinee_guard() {
    for scrutinee in ["m.lock()", "std::sync::Mutex::lock(m)"] {
        for pattern in ["Ok(..)", "Err(..)", "Ok(_) | Err(_)", "Ok(..) | Err(..)"] {
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ tick().await; }}, _ => {{}} }} }}"
            );
            assert_eq!(
                rule_ids(&report(&source, false)),
                vec![RuleId::ScbRuntime007],
                "{source}"
            );
            let source = format!(
                "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{}}, _ => {{}} }}; tick().await; }}"
            );
            assert!(report(&source, false).findings.is_empty(), "{source}");
        }
    }
}

#[test]
fn dropping_nested_ref_patterns_does_not_release_the_scrutinee_guard() {
    for (scrutinee, pattern, name) in [
        ("m.lock()", "Ok(ref g)", "g"),
        ("m.lock()", "Err(ref p)", "p"),
        ("m.try_lock()", "Ok(ref g)", "g"),
        ("m.try_lock()", "Err(TryLockError::Poisoned(ref p))", "p"),
        ("std::sync::Mutex::lock(m)", "Ok(ref g)", "g"),
        ("std::sync::Mutex::lock(m)", "Err(ref p)", "p"),
        (
            "std::sync::Mutex::try_lock(m)",
            "Err(TryLockError::Poisoned(ref p))",
            "p",
        ),
    ] {
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ drop({name}); tick().await; }}, _ => {{}} }} }}"
        );
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{source}"
        );
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match {scrutinee} {{ {pattern} => {{ drop({name}); }}, _ => {{}} }}; tick().await; }}"
        );
        assert!(report(&source, false).findings.is_empty(), "{source}");
    }
}

#[test]
fn or_patterns_preserve_named_and_borrowed_guard_ownership() {
    for (pattern, release, expected) in [
        ("g @ Ok(_) | g @ Err(_)", "drop(g);", false),
        ("g @ Ok(_) | g @ Err(_)", "", true),
        ("Ok(ref g) | Ok(ref g)", "drop(g);", true),
    ] {
        let source = format!(
            "async fn f(m: &std::sync::Mutex<()>) {{ match m.lock() {{ {pattern} => {{ {release} tick().await; }}, _ => {{}} }} }}"
        );
        assert_eq!(
            report(&source, false).findings.len(),
            usize::from(expected),
            "{source}"
        );
    }
}

#[test]
fn module_and_function_aliases_do_not_collide_in_either_declaration_order() {
    let imports = "use std::thread::sleep_until as tick; use tokio::time::sleep as nap; use futures::executor::block_on as run; use std::sync::Mutex as Lock;";
    let ordinary = "use custom::{tick, nap, run, Lock};";
    let body = "tick(()); nap(()); run(async {}); let m=Lock::new(()); let guard=m.lock().unwrap(); future.await;";
    for (checked, plain) in [
        (
            format!("mod a {{ {imports} async fn checked(){{{body}}} }}"),
            format!("mod b {{ {ordinary} async fn plain(){{{body}}} }}"),
        ),
        (
            format!("async fn checked(){{{imports}{body}}}"),
            format!("async fn plain(){{{ordinary}{body}}}"),
        ),
    ] {
        for source in [format!("{checked}{plain}"), format!("{plain}{checked}")] {
            let found = report(&source, true);
            assert_eq!(
                rule_ids(&found),
                vec![
                    RuleId::ScbRuntime004,
                    RuleId::ScbRuntime005,
                    RuleId::ScbRuntime006,
                    RuleId::ScbRuntime007
                ],
                "{source}"
            );
            assert!(
                found.findings.iter().all(|finding| finding
                    .node_ids
                    .iter()
                    .all(|id| id.as_str().ends_with("::checked"))),
                "{source}"
            );
        }
    }
}

#[test]
fn file_test_marker_exempts_legacy_runtime_rules_too() {
    let source = "fn f(cv: &Condvar) { cv.wait(guard); cv.wait_timeout(guard, duration); std::thread::sleep(duration); }";
    assert_eq!(
        rule_ids(&report(source, true)),
        vec![
            RuleId::ScbRuntime001,
            RuleId::ScbRuntime002,
            RuleId::ScbRuntime003
        ]
    );
    assert!(
        report(&format!("#![cfg(test)]\n{source}"), true)
            .findings
            .is_empty()
    );
}

#[test]
fn allowance_diagnostics_retain_typed_codes_and_source_positions() {
    let source: syn::File = syn::parse_str("#[sc_lint(allow(SCB-RUNTIME-999), reason=\"test\")] fn f() {}\n#[sc_lint(allow(SCB-RUNTIME-005))] fn g() {}") .unwrap();
    let errors = crate::practices::allowances::Allowances::collect(&source)
        .err()
        .unwrap();
    assert_eq!(errors.diagnostics.len(), 2);
    assert_eq!(errors.diagnostics[0].position.line, 1);
    assert_eq!(
        errors.diagnostics[0].directive.code(),
        sc_lint_directives::DirectiveCode::UnknownRule.qualified()
    );
    assert_eq!(errors.diagnostics[1].position.line, 2);
    assert_eq!(
        errors.diagnostics[1].directive.code(),
        sc_lint_directives::DirectiveCode::MissingReason.qualified()
    );
    assert!(
        errors
            .to_string()
            .contains(sc_lint_directives::DirectiveCode::UnknownRule.recovery())
    );
}

#[test]
fn qualified_impls_resolve_fields_in_the_declaring_module_only() {
    let declaration = "mod a { pub struct S { pub m: std::sync::Mutex<()> } }";
    let ordinary = "mod b { pub struct S { pub m: Custom } } impl b::S { async fn ordinary(&self){let g=self.m.lock().unwrap();future.await;} }";
    for owner in ["a::S", "crate::a::S"] {
        for source in [
            format!(
                "{declaration}{ordinary} impl {owner} {{async fn checked(&self){{let g=self.m.lock().unwrap();future.await;}}}}"
            ),
            format!(
                "{ordinary}{declaration} impl {owner} {{async fn checked(&self){{let g=self.m.lock().unwrap();future.await;}}}}"
            ),
        ] {
            let found = report(&source, false);
            assert_eq!(rule_ids(&found), vec![RuleId::ScbRuntime007], "{source}");
            assert!(
                found.findings[0].node_ids[0]
                    .as_str()
                    .ends_with("::checked")
            );
        }
    }
}

#[test]
fn explicit_parent_imports_keep_runtime_aliases_available_in_child_modules() {
    let imports = "use std::thread::sleep_until as tick; use tokio::time::sleep as nap; use futures::executor::block_on as run; use std::sync::Mutex as Lock;";
    let body = "tick(());nap(());run(async{});let m=Lock::new(());let guard=m.lock().unwrap();future.await;";
    for inherited in [
        "use super::*;",
        "use super::{tick, nap, run, Lock};",
        "use crate::{tick, nap, run, Lock};",
    ] {
        let source = format!("{imports} mod child {{{inherited} async fn checked(){{{body}}}}}");
        assert_eq!(
            rule_ids(&report(&source, true)),
            vec![
                RuleId::ScbRuntime004,
                RuleId::ScbRuntime005,
                RuleId::ScbRuntime006,
                RuleId::ScbRuntime007
            ],
            "{source}"
        );
    }
}

#[test]
fn parent_aliases_keep_their_definition_scope_across_nested_modules() {
    let source = "use tokio::time::sleep as nap; mod a { use super::nap; mod b { use super::*; async fn f(){nap(()).await;} } }";
    assert_eq!(
        rule_ids(&report(source, false)),
        vec![RuleId::ScbRuntime005]
    );
}

#[test]
fn qualified_003_allow_is_scoped_to_the_annotated_function() {
    let source = r#"
#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-003), reason = "Bounded synchronous compatibility polling")]
fn allowed() {
    std::thread::sleep(interval);
    fn nested() { std::thread::sleep(interval); }
}
fn sibling() { std::thread::sleep(interval); }
"#;
    let found = report(source, true);
    assert_eq!(
        rule_ids(&found),
        vec![RuleId::ScbRuntime003, RuleId::ScbRuntime003]
    );
    let labels = found
        .findings
        .iter()
        .flat_map(|finding| &finding.node_ids)
        .map(|node| node.as_str().rsplit("::").next().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        labels,
        std::collections::BTreeSet::from(["nested", "sibling"])
    );
}

#[test]
fn qualified_003_allow_requires_a_reason() {
    let fixture = tokio_fixture(
        "#[sc_lint_attributes::sc_lint(allow(SCB-RUNTIME-003))] fn missing_reason() { std::thread::sleep(interval); }",
        TOKIO,
    );
    let error = analyze_workspace(&analyze_options(&fixture))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(sc_lint_directives::DirectiveCode::MissingReason.qualified()),
        "{error}"
    );
    assert!(error.contains("at line 1"), "{error}");
    assert!(error.contains("recovery:"), "{error}");
}

#[test]
fn imported_root_statics_preserve_mutex_identity() {
    let source = r#"
        static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
        mod a { use crate::M; async fn f() { let guard = M.lock().unwrap(); work().await; } }
        mod b { use super::M; async fn f() { let guard = M.lock().unwrap(); work().await; } }
    "#;
    assert_eq!(
        rule_ids(&report(source, false)),
        vec![RuleId::ScbRuntime007; 2]
    );
}

#[test]
fn sleep_imports_do_not_leak_between_modules_or_functions() {
    let blocking = "mod a { use std::thread::sleep; fn f(){ sleep(()); } }";
    let timer = "mod b { use tokio::time::sleep; fn f(){ sleep(()); } }";
    for source in [
        format!("{blocking}{timer}"),
        format!("{timer}{blocking}"),
        "fn a(){use std::thread::sleep; sleep(());} fn b(){use tokio::time::sleep; sleep(());}"
            .into(),
    ] {
        assert_eq!(
            rule_ids(&report(&source, true)),
            vec![RuleId::ScbRuntime003, RuleId::ScbRuntime005]
        );
    }
}

#[test]
fn all_wait_rules_share_compound_and_framework_test_exemptions() {
    let body = "fn f(c:&std::sync::Condvar,m:&std::sync::Mutex<()>){let g=m.lock().unwrap();c.wait(g);c.wait_timeout(g,d);}";
    assert_eq!(
        rule_ids(&report(body, false)),
        vec![RuleId::ScbRuntime001, RuleId::ScbRuntime002]
    );
    for attr in [
        "#[cfg(all(test,unix))]",
        "#[tokio::test]",
        "#[async_std::test]",
    ] {
        assert!(
            report(&format!("{attr}{body}"), false).findings.is_empty(),
            "{attr}"
        );
    }
}

#[test]
fn runtime_allowances_reject_non_function_placement() {
    for item in ["mod m {}", "struct S;", "impl S {}", "const C: () = ();"] {
        let source = format!("#[sc_lint(allow(SCB-RUNTIME-003),reason=\"deliberate\")] {item}");
        let fixture = tokio_fixture(&source, TOKIO);
        let error = analyze_workspace(&analyze_options(&fixture)).unwrap_err();
        assert!(error.to_string().contains("UNSUPPORTED_ITEM"), "{error}");
    }
}

#[test]
fn runtime_reports_every_unknown_owned_rule() {
    let fixture = tokio_fixture(
        "#[sc_lint(allow(SCB-RUNTIME-998,SCB-RUNTIME-999),reason=\"typos\")]fn f(){}",
        TOKIO,
    );
    let error = analyze_workspace(&analyze_options(&fixture))
        .unwrap_err()
        .to_string();
    assert_eq!(
        error
            .matches(sc_lint_directives::DirectiveCode::UnknownRule.qualified())
            .count(),
        2,
        "{error}"
    );
    assert!(error.contains("valid rule ids:"));
}

#[test]
fn typed_runtime_inventory_matches_catalog() {
    assert_eq!(
        RuleId::ALL.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        RUNTIME_CATALOG
            .rules
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn parent_and_current_module_imports_preserve_static_identity() {
    let source = r#"
        mod a { pub static M: std::sync::Mutex<()> = std::sync::Mutex::new(()); }
        static R: std::sync::Mutex<()> = std::sync::Mutex::new(());
        mod b { use super::a::M; async fn f(){let g=M.lock().unwrap();tick().await;} }
        mod c { use crate::a::M; async fn f(){let g=M.lock().unwrap();tick().await;} }
        mod d { use crate::R; async fn f(){let g=R.lock().unwrap();tick().await;} }
        mod e { use super::R; async fn f(){let g=R.lock().unwrap();tick().await;} }
    "#;
    assert_eq!(
        rule_ids(&report(source, false)),
        vec![RuleId::ScbRuntime007; 4]
    );
    for (declarations, imports) in [
        ("", "use super::super::a::M;"),
        (
            "mod a {pub static M: std::sync::Mutex<()> = std::sync::Mutex::new(());}",
            "use self::a::M;",
        ),
        ("", "use super::super::a::*;"),
        (
            "mod a {pub static M: std::sync::Mutex<()> = std::sync::Mutex::new(());}",
            "use self::a::*;",
        ),
        ("", "use crate::a::*;"),
    ] {
        let source = format!(
            "mod a {{pub static M: std::sync::Mutex<()> = std::sync::Mutex::new(());}} mod outer {{mod b {{{declarations} {imports} async fn f(){{let g=M.lock().unwrap();tick().await;}}}}}}"
        );
        assert_eq!(
            rule_ids(&report(&source, false)),
            vec![RuleId::ScbRuntime007],
            "{imports}"
        );
    }
    let source = "mod outer { mod a {pub static M: std::sync::Mutex<()> = std::sync::Mutex::new(());} mod b {use super::a::M;async fn f(){let g=M.lock().unwrap();tick().await;}}}";
    assert_eq!(
        rule_ids(&report(source, false)),
        vec![RuleId::ScbRuntime007]
    );
}
