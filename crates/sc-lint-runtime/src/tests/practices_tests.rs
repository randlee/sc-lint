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
            "DIRECTIVE.SITE_ALLOW.MISSING_REASON",
        ),
        (
            "allow(SCB-RUNTIME-005), reason=\"  \"",
            "DIRECTIVE.SITE_ALLOW.EMPTY_REASON",
        ),
        (
            "allow(SCB-RUNTIME-999), reason=\"unknown\"",
            "DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE",
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
