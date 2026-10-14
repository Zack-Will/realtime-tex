//! acmart (the ACM class) under LuaLaTeX. The class checks at the end of the document that
//! `\section` and the other headings are the ones it defined, and the capture must not trip that
//! check while a document that does redefine a heading is still reported. Skipped without
//! lualatex or acmart.cls.

use rtex_core::capture::run_capture;
use rtex_core::session::parse_log;
use rtex_core::texlive::TexLive;

const PAR: &str = "Ordinary words fill this paragraph so that it runs over a few lines of the \
                   narrow column of the conference layout here.";

/// Captures an acmart `sigconf` document (`preamble` before `\begin{document}`, `body` inside
/// it) and returns the error messages of its log.
fn capture_errors(name: &str, preamble: &str, body: &str) -> Option<Vec<String>> {
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
    let main = format!(
        "\\documentclass[sigconf,nonacm]{{acmart}}\n\\title{{Headings}}\n\
         \\author{{An Author}}\n\\affiliation{{\\institution{{An Institution}}\\country{{A Country}}}}\n\
         {preamble}\n\\begin{{document}}\n\\maketitle\n\
         \\section{{Introduction}}\n{PAR}\n\n\\subsection{{Design}}\n{PAR}\n\n\
         \\subsubsection{{Paths}}\n{PAR}\n\n\\paragraph{{Costs}} {PAR}\n\n{body}\n\\end{{document}}\n"
    );
    std::fs::write(project.join("main.tex"), &main).unwrap();
    let cap = run_capture(&tl, &project, "main.tex", &root.join("cap"), true).unwrap();
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
