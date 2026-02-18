use printpdf::{Color, FontId, Mm, Op, PdfPage, Pt, Rgb, TextItem, graphics::Point};

/// A styled text span within a line.
pub struct Span {
    pub text: String,
    pub font_id: FontId,
    pub size: Pt,
    pub color: Color,
}

/// Font set for the four standard variants.
#[derive(Clone)]
pub struct FontSet {
    pub regular: FontId,
    pub bold: FontId,
    pub italic: FontId,
    pub bold_italic: FontId,
}

/// Builds PDF pages with simple top-to-bottom text layout.
///
/// Coordinates: printpdf uses bottom-left origin. We track `y` from the top
/// of the usable area (top margin) downward. When converting to printpdf
/// coordinates we do: `pdf_y = page_height - margin - y`.
pub struct PageBuilder {
    pages: Vec<PdfPage>,
    current_ops: Vec<Op>,
    y: f32,
    page_width: Mm,
    page_height: Mm,
    margin: Mm,
    line_height: f32,
    page_count: usize,
    pending_break: bool,
    fonts: FontSet,
}

impl PageBuilder {
    pub fn new(
        page_width: Mm,
        page_height: Mm,
        margin: Mm,
        line_height: f32,
        fonts: FontSet,
        starting_page: usize,
    ) -> Self {
        let mut builder = Self {
            pages: Vec::new(),
            current_ops: Vec::new(),
            y: 0.0,
            page_width,
            page_height,
            margin,
            line_height,
            page_count: starting_page.saturating_sub(1),
            pending_break: false,
            fonts,
        };
        builder.start_new_page();
        builder
    }

    /// The page number currently being written, accounting for a pending deferred break.
    pub fn current_page(&self) -> usize {
        if self.pending_break { self.page_count + 1 } else { self.page_count }
    }

    fn usable_height(&self) -> f32 {
        self.page_height.into_pt().0 - 2.0 * self.margin.into_pt().0
    }

    fn remaining(&self) -> f32 {
        self.usable_height() - self.y
    }

    fn pdf_y(&self) -> Pt {
        Pt(self.page_height.into_pt().0 - self.margin.into_pt().0 - 12.0 - self.y)
    }

    fn left_x(&self) -> Pt {
        self.margin.into_pt()
    }

    fn start_new_page(&mut self) {
        if !self.current_ops.is_empty() {
            self.pages.push(PdfPage::new(
                self.page_width,
                self.page_height,
                std::mem::take(&mut self.current_ops),
            ));
        }

        self.page_count += 1;
        self.y = 0.0;

        let header_text = format!("- {} -", self.page_count);
        let header_x = self.page_width.into_pt().0 / 2.0 - (header_text.len() as f32 * 2.5);
        let header_y = self.page_height.into_pt().0 - self.margin.into_pt().0 + 2.0;
        let header_font = self.fonts.regular.clone();

        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: Pt(header_x),
                    y: Pt(header_y),
                },
            },
            Op::SetFillColor {
                col: Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)),
            },
            Op::SetFontSize {
                size: Pt(7.0),
                font: header_font.clone(),
            },
            Op::WriteText {
                items: vec![TextItem::Text(header_text)],
                font: header_font,
            },
            Op::EndTextSection,
        ]);
    }

    /// Flush a deferred page break: start the new page now.
    fn flush_break(&mut self) {
        if self.pending_break {
            self.pending_break = false;
            self.start_new_page();
        }
    }

    pub fn ensure_space(&mut self, needed_pt: f32) {
        self.flush_break();
        if self.remaining() < needed_pt {
            self.start_new_page();
        }
    }

    /// Mark a section boundary. The new page is created lazily on the next write,
    /// so finish() never produces a trailing empty page.
    pub fn page_break(&mut self) {
        self.pending_break = true;
    }

    pub fn write_line(&mut self, spans: &[Span]) {
        self.ensure_space(self.line_height);

        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: self.left_x(),
                    y: self.pdf_y(),
                },
            },
        ]);

        self.current_ops.extend(spans.iter().flat_map(|span| {
            [
                Op::SetFillColor {
                    col: span.color.clone(),
                },
                Op::SetFontSize {
                    size: span.size,
                    font: span.font_id.clone(),
                },
                Op::WriteText {
                    items: vec![TextItem::Text(span.text.clone())],
                    font: span.font_id.clone(),
                },
            ]
        }));

        self.current_ops.push(Op::EndTextSection);
        self.y += self.line_height;
    }

    pub fn vertical_space(&mut self, pt: f32) {
        self.y += pt;
    }

    pub fn write_centered(&mut self, text: &str, font_id: &FontId, size: Pt, color: Color) {
        self.ensure_space(size.0 + 4.0);

        let text_width = text.len() as f32 * size.0 * 0.6;
        let x = (self.page_width.into_pt().0 - text_width) / 2.0;

        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: Pt(x.max(0.0)),
                    y: self.pdf_y(),
                },
            },
            Op::SetFillColor { col: color },
            Op::SetFontSize {
                size,
                font: font_id.clone(),
            },
            Op::WriteText {
                items: vec![TextItem::Text(text.to_string())],
                font: font_id.clone(),
            },
            Op::EndTextSection,
        ]);

        self.y += size.0 + 4.0;
    }

    pub fn write_line_centered(&mut self, spans: &[Span]) {
        self.ensure_space(self.line_height);
        let y = self.pdf_y();

        let total_width: f32 = spans.iter().map(|s| s.text.len() as f32 * s.size.0 * 0.6).sum();
        let x = ((self.page_width.into_pt().0 - total_width) / 2.0).max(0.0);

        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point { x: Pt(x), y },
            },
        ]);
        self.current_ops.extend(spans.iter().flat_map(|span| {
            [
                Op::SetFillColor {
                    col: span.color.clone(),
                },
                Op::SetFontSize {
                    size: span.size,
                    font: span.font_id.clone(),
                },
                Op::WriteText {
                    items: vec![TextItem::Text(span.text.clone())],
                    font: span.font_id.clone(),
                },
            ]
        }));
        self.current_ops.push(Op::EndTextSection);
        self.y += self.line_height;
    }

    pub fn write_line_justified(&mut self, left: &[Span], right: &[Span]) {
        self.ensure_space(self.line_height);
        let y = self.pdf_y();

        // Left-aligned spans
        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: self.left_x(),
                    y,
                },
            },
        ]);
        self.current_ops.extend(left.iter().flat_map(|span| {
            [
                Op::SetFillColor {
                    col: span.color.clone(),
                },
                Op::SetFontSize {
                    size: span.size,
                    font: span.font_id.clone(),
                },
                Op::WriteText {
                    items: vec![TextItem::Text(span.text.clone())],
                    font: span.font_id.clone(),
                },
            ]
        }));
        self.current_ops.push(Op::EndTextSection);

        // Right-aligned spans
        let right_width: f32 = right
            .iter()
            .map(|s| s.text.len() as f32 * s.size.0 * 0.6)
            .sum();
        let right_x = self.page_width.into_pt().0 - self.margin.into_pt().0 - right_width;

        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: Pt(right_x.max(0.0)),
                    y,
                },
            },
        ]);
        self.current_ops.extend(right.iter().flat_map(|span| {
            [
                Op::SetFillColor {
                    col: span.color.clone(),
                },
                Op::SetFontSize {
                    size: span.size,
                    font: span.font_id.clone(),
                },
                Op::WriteText {
                    items: vec![TextItem::Text(span.text.clone())],
                    font: span.font_id.clone(),
                },
            ]
        }));
        self.current_ops.push(Op::EndTextSection);

        self.y += self.line_height;
    }

    pub fn font(&self, bold: bool, italic: bool) -> &FontId {
        match (bold, italic) {
            (true, true) => &self.fonts.bold_italic,
            (true, false) => &self.fonts.bold,
            (false, true) => &self.fonts.italic,
            (false, false) => &self.fonts.regular,
        }
    }

    pub fn finish(mut self) -> Vec<PdfPage> {
        if !self.current_ops.is_empty() {
            self.pages.push(PdfPage::new(
                self.page_width,
                self.page_height,
                self.current_ops,
            ));
        }
        self.pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font_set() -> (printpdf::PdfDocument, FontSet) {
        let mut doc = printpdf::PdfDocument::new("test");

        let load =
            |bytes: &[u8]| printpdf::ParsedFont::from_bytes(bytes, 0, &mut Vec::new()).unwrap();

        let fonts = FontSet {
            regular: doc.add_font(&load(include_bytes!(
                "../../fonts/JetBrainsMono-Regular.ttf"
            ))),
            bold: doc.add_font(&load(include_bytes!("../../fonts/JetBrainsMono-Bold.ttf"))),
            italic: doc.add_font(&load(include_bytes!(
                "../../fonts/JetBrainsMono-Italic.ttf"
            ))),
            bold_italic: doc.add_font(&load(include_bytes!(
                "../../fonts/JetBrainsMono-BoldItalic.ttf"
            ))),
        };

        (doc, fonts)
    }

    fn black() -> Color {
        Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None))
    }

    #[test]
    fn builder_creates_at_least_one_page() {
        let (_doc, fonts) = test_font_set();
        let pages = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1).finish();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn write_line_adds_content() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line(&[Span {
            text: "hello".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        let pages = builder.finish();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].ops.len() > 2);
    }

    #[test]
    fn page_break_creates_new_page() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line(&[Span {
            text: "page 1".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        builder.page_break();
        builder.write_line(&[Span {
            text: "page 2".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        assert_eq!(builder.finish().len(), 2);
    }

    #[test]
    fn trailing_page_break_does_not_add_empty_page() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line(&[Span {
            text: "content".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        builder.page_break();
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn write_centered_works() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_centered("Title", &fonts.regular, Pt(28.0), black());
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn many_lines_cause_page_overflow() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        (0..200).for_each(|_| {
            builder.write_line(&[Span {
                text: "line".into(),
                font_id: fonts.regular.clone(),
                size: Pt(8.0),
                color: black(),
            }]);
        });
        assert!(builder.finish().len() > 1);
    }
}
