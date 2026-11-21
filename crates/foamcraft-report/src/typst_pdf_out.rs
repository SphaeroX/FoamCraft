// SPDX-License-Identifier: GPL-3.0-or-later
//! PDF through Typst: an in-memory world with the template, the report data as JSON, the images
//! and the embedded Libertinus Serif font (SIL OFL 1.1, `assets/fonts`).

use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Frame, FrameItem};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

use crate::ReportModel;

const TEMPLATE: &str = include_str!("../data/report.typ");
const FONTS: [&[u8]; 3] = [
    include_bytes!("../../../assets/fonts/LibertinusSerif-Regular.otf"),
    include_bytes!("../../../assets/fonts/LibertinusSerif-Bold.otf"),
    include_bytes!("../../../assets/fonts/LibertinusSerif-Italic.otf"),
];

fn id(path: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("valid virtual path"),
    ))
}

struct ReportWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: Source,
    files: HashMap<FileId, Bytes>,
}

impl ReportWorld {
    fn new(m: &ReportModel) -> Result<Self, String> {
        let fonts: Vec<Font> = FONTS
            .iter()
            .flat_map(|data| Font::iter(Bytes::new(data.to_vec())))
            .collect();
        let main_id = id("/main.typ");
        let mut data = serde_json::to_value(m).map_err(|e| e.to_string())?;
        data["logo"] = serde_json::json!(m.logo_png.is_some());
        data["chart"] = serde_json::json!(m.convergence.chart_svg.is_some());
        let mut files = HashMap::new();
        files.insert(
            id("/data.json"),
            Bytes::new(serde_json::to_vec(&data).map_err(|e| e.to_string())?),
        );
        if let Some(svg) = &m.convergence.chart_svg {
            files.insert(id("/chart.svg"), Bytes::new(svg.clone().into_bytes()));
        }
        if let Some(logo) = &m.logo_png {
            files.insert(id("/logo.png"), Bytes::new(logo.clone()));
        }
        for v in &m.views {
            files.insert(
                id(&format!("/view-{}.png", v.key)),
                Bytes::new(v.png.clone()),
            );
        }
        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
            main: Source::new(main_id, TEMPLATE.to_string()),
            files,
        })
    }
}

impl World for ReportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::NotSource)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main.id() {
            return Ok(Bytes::from_string(self.main.text().to_string()));
        }
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.get().vpath().get_with_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        // The report shows its own date; Typst's clock is not used.
        Datetime::from_ymd(2026, 1, 1)
    }
}

fn compile(m: &ReportModel) -> Result<PagedDocument, String> {
    let world = ReportWorld::new(m)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
}

/// The report as PDF.
pub fn pdf(m: &ReportModel) -> Result<Vec<u8>, String> {
    let doc = compile(m)?;
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.message.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn collect(frame: &Frame, out: &mut String) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Text(t) => {
                out.push_str(&t.text);
                out.push(' ');
            }
            FrameItem::Group(g) => collect(&g.frame, out),
            _ => {}
        }
    }
}

/// Page count and the laid-out text of the PDF (for checks of its content).
pub fn pdf_text(m: &ReportModel) -> Result<(usize, String), String> {
    let doc = compile(m)?;
    let mut text = String::new();
    for page in doc.pages() {
        collect(&page.frame, &mut text);
        text.push('\n');
    }
    Ok((doc.pages().len(), text))
}
