//! acmart (the ACM class) under LuaLaTeX. The class checks at the end of the document that
//! `\section` and the other headings are the ones it defined, and it switches to two columns in
//! `\maketitle`, which the live engine never runs. Skipped without lualatex or acmart.cls.

use std::path::PathBuf;

use rtex_core::capture::{run_capture, CaptureResult};
use rtex_core::eligibility::{check_engine_unit, classify_source, Policy, Reason};
use rtex_core::engine::FastServer;
use rtex_core::layout::LayoutStore;
use rtex_core::session::parse_log;
use rtex_core::texlive::TexLive;

const PAR: &str = "Ordinary words fill this paragraph so that it runs over a few lines of the \
                   narrow column of the conference layout here.";

/// An acmart `sigconf` document with `preamble` before `\begin{document}` and `body` after the
/// title and four levels of headings.
fn document(preamble: &str, body: &str) -> String {
    format!(
        "\\documentclass[sigconf,nonacm]{{acmart}}\n\\title{{Headings}}\n\
         \\author{{An Author}}\n\\affiliation{{\\institution{{An Institution}}\\country{{A Country}}}}\n\
         {preamble}\n\\begin{{document}}\n\\maketitle\n\
         \\section{{Introduction}}\n{PAR}\n\n\\subsection{{Design}}\n{PAR}\n\n\
         \\subsubsection{{Paths}}\n{PAR}\n\n\\paragraph{{Costs}} {PAR}\n\n{body}\n\\end{{document}}\n"
    )
}

/// Captures `main` as `main.tex` of a fresh project. Returns the TeX Live, the scratch root (to
/// remove when done), the project directory and the capture.
fn capture(name: &str, main: &str) -> Option<(TexLive, PathBuf, PathBuf, CaptureResult)> {
    let Ok(tl) = TexLive::discover() else {
        eprintln!("SKIP: no lualatex");
        return None;
    };
    let have_class = std::process::Command::new("kpsewhich")
        .arg("acmart.cls")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    if !have_class {
        eprintln!("SKIP: no acmart.cls");
        return None;
    }
    let root = std::env::temp_dir().join(format!("rtex-acmart-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("main.tex"), main).unwrap();
    let cap = run_capture(&tl, &project, "main.tex", &root.join("cap"), true).unwrap();
    Some((tl, root, project, cap))
}

/// The error messages of the capture's log.
fn capture_errors(name: &str, preamble: &str, body: &str) -> Option<Vec<String>> {
    let (_, root, _, cap) = capture(name, &document(preamble, body))?;
    let errors = parse_log(&cap.log)
        .into_iter()
        .filter(|d| d.severity == "error")
        .map(|d| d.message)
        .collect();
    let _ = std::fs::remove_dir_all(&root);
    Some(errors)
}

/// The capture hooks the headings to start a unit at each of them. LaTeX installs such hooks
/// when the document begins, after acmart saved the headings to compare them at the end, so the
/// class took the hooks for a redefinition: "An attempt to redefine \section detected", once per
/// heading, in every capture and every background pass.
#[test]
fn the_capture_does_not_trip_acmarts_heading_check() {
    let Some(errors) = capture_errors("plain", "", "") else {
        return;
    };
    assert!(errors.is_empty(), "{errors:?}");
}

/// A document that redefines a heading itself, in the preamble or in the body, is still
/// reported by the class.
#[test]
fn a_heading_the_document_redefines_is_still_reported() {
    let redefine = "\\renewcommand\\section[1]{#1}";
    for (name, preamble, body) in [("preamble", redefine, ""), ("body", "", redefine)] {
        let Some(errors) = capture_errors(name, preamble, body) else {
            return;
        };
        assert!(
            errors
                .iter()
                .any(|e| e.contains("An attempt to redefine \\section detected")),
            "{name}: {errors:?}"
        );
        assert!(
            !errors.iter().any(|e| e.contains("redefine \\subsection")),
            "{name}: {errors:?}"
        );
    }
}

/// A float of the document: `env:<name>`, why it may not go live (the allow-list's checks, as
/// `rtex verify` applies them) and, when it may, how its live compile compares with its rows on
/// the pages.
type Float = (String, Vec<Reason>, Option<Result<(), String>>);

/// The floats of an acmart document with `body`, in document order.
fn floats(name: &str, body: &str) -> Option<Vec<Float>> {
    let main = document("", body);
    let (tl, root, project, cap) = capture(name, &main)?;
    let preamble = rtex_core::split_preamble(&main).unwrap().0.to_string();
    let policy = Policy::from_preamble(&preamble, &[], &[]);
    let (store, files) = LayoutStore::offline(&cap, &project, "main.tex", &policy).unwrap();
    let mut srv =
        FastServer::spawn(&tl, &project, &root.join("serve"), &preamble, 1, None).unwrap();
    let mut out = Vec::new();
    for (eu, id) in store.mapped_units() {
        if eu.rows() == 0 || eu.kind() != "env" {
            continue;
        }
        let c = &eu.captured;
        let src = files.fast_source(id).unwrap();
        let mut reasons = classify_source(&src, &policy).1;
        reasons.extend(check_engine_unit(
            &c.kind,
            &c.everypar,
            eu.has_context(),
            eu.rows(),
            &eu.flags,
            c.name.as_deref().filter(|_| c.runin),
        ));
        let verdict = reasons.is_empty().then(|| {
            srv.set_context(eu.uid, &eu.context_json()).unwrap();
            let (res, _) = srv.compile(eu.uid, &src).unwrap();
            match &res.dl {
                Some(dl) => store.probe_check(id, dl),
                None => Err(format!("no result: {}", res.status)),
            }
        });
        out.push((
            format!("env:{}", eu.name().unwrap_or_default()),
            reasons,
            verdict,
        ));
    }
    let _ = srv.shutdown();
    let _ = std::fs::remove_dir_all(&root);
    Some(out)
}

/// acmart's two columns start in `\maketitle`, so the live engine, which never runs it, idles
/// with the one-column `\columnwidth` and `\linewidth`. A float compiled live must get the
/// column geometry the document has where the float is: a `table` one column wide, a `table*`
/// as wide as the page, and `\linewidth` inside each.
#[test]
fn floats_get_the_documents_column_geometry() {
    let table = "\\centering\n\\caption{A table}\n\\begin{tabular}{cc}\nalpha & beta \\\\\n\
                 gamma & delta \\\\\n\\end{tabular}\n";
    let figure =
        "\\centering\n\\rule{0.8\\linewidth}{1cm}\n\\caption{A figure as wide as the line}\n";
    let body = format!(
        "{PAR}\n\n\\begin{{table}}[t]\n{table}\\end{{table}}\n\n{PAR}\n\n\
         \\begin{{figure}}[t]\n{figure}\\end{{figure}}\n\n{PAR}\n\n\
         \\begin{{table*}}[t]\n{table}\\end{{table*}}\n\n{PAR}\n\n\
         \\begin{{figure*}}[t]\n{figure}\\end{{figure*}}\n\n{PAR}\n"
    );
    let Some(floats) = floats("floats", &body) else {
        return;
    };
    let names: Vec<&str> = floats.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(
        names,
        ["env:table", "env:figure", "env:table*", "env:figure*"],
        "{floats:?}"
    );
    for (name, reasons, verdict) in &floats {
        assert!(reasons.is_empty(), "{name}: {reasons:?}");
        assert!(
            matches!(verdict, Some(Ok(()))),
            "{name} differs from the full run: {verdict:?}"
        );
    }
}
