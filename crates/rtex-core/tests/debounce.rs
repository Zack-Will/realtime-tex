//! Background layouts keep arriving during typing; explicit requests bypass the debounce.

use rtex_core::session::{CompileStatus, EligibilityMode};
use rtex_core::{Edit, Event, Session, SessionConfig};
use std::time::{Duration, Instant};

const MAX_WAIT: Duration = Duration::from_millis(350);
const SLACK: Duration = Duration::from_millis(400);
// typeout is outside the allow-list, so each edit schedules a background pass.
const MAIN: &str = "\\documentclass{article}\n\\begin{document}\n\
                    \\typeout{typing}Typing text.\n\\end{document}\n";

fn open(name: &str, max_wait: Option<Duration>) -> Option<(Session, std::path::PathBuf)> {
    if rtex_core::texlive::TexLive::discover().is_err() {
        eprintln!("SKIP: no lualatex");
        return None;
    }
    let root = std::env::temp_dir().join(format!("rtex-debounce-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tex"), MAIN).unwrap();
    let mut cfg = SessionConfig::new(&root, "main.tex");
    cfg.debounce = Duration::from_secs(5);
    cfg.debounce_max_wait = max_wait;
    cfg.max_passes = 1;
    cfg.eligibility = EligibilityMode::AllowList;
    let session = Session::open(cfg).unwrap();
    let (layout, _) = session.wait_for(Duration::from_secs(60), |e| {
        matches!(e, Event::LayoutUpdate { .. })
    });
    assert!(
        matches!(
            layout,
            Some(Event::LayoutUpdate {
                compile: CompileStatus::Ok,
                ..
            })
        ),
        "initial layout: {layout:?}"
    );
    session.poll(Duration::ZERO);
    Some((session, root))
}

fn type_character(session: &Session) {
    let text = session.document_text("main.tex").unwrap();
    let at = text.find('.').unwrap();
    let result = session
        .apply_edit(
            "main.tex",
            Edit {
                start_byte: at,
                end_byte: at,
                text: "x".into(),
            },
        )
        .unwrap();
    assert_eq!(result.routed, "background", "{result:?}");
}

#[test]
fn continuous_edits_have_bounded_layout_cadence() {
    let Some((session, root)) = open("cadence", Some(MAX_WAIT)) else {
        return;
    };
    let started = Instant::now();
    let typing_for = Duration::from_secs(4);
    let mut next_edit = Duration::ZERO;
    let mut last_layout = Duration::ZERO;
    let mut last_bound = MAX_WAIT + SLACK;
    let mut layouts = 0;
    let mut edits = 0;
    while started.elapsed() < typing_for {
        if started.elapsed() >= next_edit {
            type_character(&session);
            edits += 1;
            next_edit += Duration::from_millis(100);
        }
        for event in session.poll(Duration::from_millis(5)) {
            if let Event::LayoutUpdate {
                compile,
                wall_ms,
                versions,
                ..
            } = event
            {
                assert_eq!(compile, CompileStatus::Ok);
                let elapsed = started.elapsed();
                let gap = elapsed - last_layout;
                let bound = MAX_WAIT + Duration::from_millis(wall_ms) + SLACK;
                eprintln!(
                    "[cadence] layout v{} at {} ms; gap {} ms; pass {} ms; bound {} ms",
                    versions.layout_version,
                    elapsed.as_millis(),
                    gap.as_millis(),
                    wall_ms,
                    bound.as_millis()
                );
                assert!(gap <= bound, "layout gap {gap:?} exceeds {bound:?}");
                last_layout = elapsed;
                last_bound = bound;
                layouts += 1;
            }
        }
    }
    eprintln!("[cadence] {edits} edits at 100 ms intervals, {layouts} layouts during 4 s of continuous typing");
    assert!(edits >= 35, "typing was unexpectedly blocked");
    assert!(layouts >= 4, "continuous typing starved layouts");
    assert!(
        started.elapsed() - last_layout <= last_bound,
        "layouts stopped before typing ended"
    );
    session.close();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn request_layout_during_typing_bypasses_quiet_period() {
    let Some((session, root)) = open("request", None) else {
        return;
    };
    let started = Instant::now();
    let mut next_edit = Duration::ZERO;
    let mut requested = None;
    let mut layout_latency = None;
    while started.elapsed() < Duration::from_secs(2) {
        if started.elapsed() >= next_edit {
            type_character(&session);
            next_edit += Duration::from_millis(100);
        }
        if requested.is_none() && started.elapsed() >= Duration::from_millis(300) {
            assert!(
                !session
                    .poll(Duration::ZERO)
                    .iter()
                    .any(|e| matches!(e, Event::LayoutUpdate { .. })),
                "a debounced pass started before the explicit request"
            );
            requested = Some(Instant::now());
            session.request_layout();
        }
        for event in session.poll(Duration::from_millis(5)) {
            if let Event::LayoutUpdate {
                compile, wall_ms, ..
            } = event
            {
                assert_eq!(compile, CompileStatus::Ok);
                let latency = requested.expect("layout before request").elapsed();
                let bound = Duration::from_millis(wall_ms) + SLACK;
                eprintln!("[request_layout] layout after {} ms; pass {} ms; bound {} ms; quiet period 5000 ms",
                    latency.as_millis(), wall_ms, bound.as_millis());
                assert!(latency <= bound, "explicit request was debounced");
                layout_latency = Some(latency);
            }
        }
    }
    assert!(
        layout_latency.is_some(),
        "explicit request was swallowed during typing"
    );
    session.close();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_request_stays_paused_and_runs_promptly_on_resume() {
    let Some((session, root)) = open("paused", Some(MAX_WAIT)) else {
        return;
    };
    session.pause_background(true);
    type_character(&session);
    session.request_layout();
    let paused_at = Instant::now();
    while paused_at.elapsed() < Duration::from_millis(700) {
        assert!(
            !session
                .poll(Duration::from_millis(10))
                .iter()
                .any(|e| matches!(e, Event::LayoutUpdate { .. })),
            "pass started while paused"
        );
    }
    let resumed = Instant::now();
    session.pause_background(false);
    let (layout, _) = session.wait_for(Duration::from_secs(3), |e| {
        matches!(e, Event::LayoutUpdate { .. })
    });
    let Some(Event::LayoutUpdate {
        compile, wall_ms, ..
    }) = layout
    else {
        panic!("no layout after resume")
    };
    assert_eq!(compile, CompileStatus::Ok);
    let bound = Duration::from_millis(wall_ms) + SLACK;
    eprintln!(
        "[pause] layout {} ms after resume; pass {} ms; bound {} ms",
        resumed.elapsed().as_millis(),
        wall_ms,
        bound.as_millis()
    );
    assert!(resumed.elapsed() <= bound);
    session.close();
    std::fs::remove_dir_all(root).unwrap();
}
