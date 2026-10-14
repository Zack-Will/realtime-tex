//! Images in display lists name their file: an image item carries the index its image node has,
//! and the file behind each index comes from graphicx (the capture and the live engine record
//! it as `\includegraphics` loads the file). Skipped without lualatex.

use rtex_core::capture::run_capture;
use rtex_core::eligibility::Policy;
use rtex_core::engine::FastServer;
use rtex_core::layout::LayoutStore;
use rtex_core::texlive::TexLive;
use rtex_dl::{DisplayList, ImageInfo, Item};
use std::collections::BTreeMap;

const MAIN: &str = "\\documentclass{article}\n\\usepackage{graphicx}\n\\usepackage{hyperref}\n\
                    \\begin{document}\n\
                    First image: \\includegraphics[width=1cm]{a.png}.\n\n\
                    Second image: \\includegraphics[width=2cm]{b.png}.\n\n\
                    The first image again: \\includegraphics[width=3cm]{a.png}.\n\
                    \\end{document}\n";

/// The files of the image items of `dl`, in order, looked up in `images`.
fn files(dl: &DisplayList, images: &BTreeMap<String, ImageInfo>) -> Vec<String> {
    dl.lines
        .iter()
        .flat_map(|l| &l.items)
        .chain(&dl.other)
        .filter_map(|it| match it {
            Item::Image { index, .. } => Some(
                images
                    .get(&index.to_string())
                    .map_or(format!("no file for index {index}"), |i| i.file.clone()),
            ),
            _ => None,
        })
        .collect()
}

/// The index used to come from `\lastsavedimageresourceindex`, which is the resource's PDF object
/// number (hyperref's objects come first, so it is not the node's index) and, when graphicx
/// reuses `a.png`'s resource for the third image, still the one saved for `b.png`. The capture's
/// page and each live compile must give `a.png`, `b.png`, `a.png`.
#[test]
fn every_image_names_its_file() {
    let Ok(tl) = TexLive::discover() else {
        eprintln!("SKIP: no lualatex");
        return;
    };
    let root = std::env::temp_dir().join(format!("rtex-images-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let png = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/corpus/layout/figure.png");
    for name in ["a.png", "b.png"] {
        std::fs::copy(&png, project.join(name)).unwrap();
    }
    std::fs::write(project.join("main.tex"), MAIN).unwrap();
    let cap = run_capture(&tl, &project, "main.tex", &root.join("cap"), true).unwrap();
    let page = cap.page(1).unwrap();
    assert_eq!(files(&page, &page.images), ["a.png", "b.png", "a.png"]);

    let preamble = rtex_core::split_preamble(MAIN).unwrap().0.to_string();
    let policy = Policy::from_preamble(&preamble, &[], &[]);
    let (store, sources) = LayoutStore::offline(&cap, &project, "main.tex", &policy).unwrap();
    let mut srv =
        FastServer::spawn(&tl, &project, &root.join("serve"), &preamble, 1, None).unwrap();
    let mut live = Vec::new();
    for (eu, id) in store.mapped_units() {
        if eu.rows() == 0 {
            continue;
        }
        srv.set_context(eu.uid, &eu.context_json()).unwrap();
        let (res, _) = srv
            .compile(eu.uid, &sources.fast_source(id).unwrap())
            .unwrap();
        live.extend(files(res.dl.as_ref().unwrap(), &res.images));
    }
    let _ = srv.shutdown();
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(live, ["a.png", "b.png", "a.png"]);
}
