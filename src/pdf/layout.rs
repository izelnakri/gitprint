use printpdf::{
    Actions, BorderArray, Color, ColorArray, FontId, Line, LinePoint, LinkAnnotation, Mm, Op,
    PaintMode, PdfFontHandle, PdfPage, Polygon, PolygonRing, Pt, Rect, Rgb, TextItem, WindingOrder,
    graphics::Point,
};

/// A styled text span within a line.
pub struct Span {
    /// The text content of this span.
    pub text: String,
    /// The font to use for this span.
    pub font_id: FontId,
    /// The font size in points.
    pub size: Pt,
    /// The fill color for the text.
    pub color: Color,
}

/// Font set for the four standard variants.
#[derive(Clone)]
pub struct FontSet {
    /// Regular (upright, normal weight) font handle.
    pub regular: FontId,
    /// Bold (upright, bold weight) font handle.
    pub bold: FontId,
    /// Italic (oblique, normal weight) font handle.
    pub italic: FontId,
    /// Bold-italic font handle.
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
    /// Creates a new `PageBuilder` with the given page dimensions, margin, line height, and fonts.
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
        if self.pending_break {
            self.page_count + 1
        } else {
            self.page_count
        }
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
            Op::SetFont {
                size: Pt(7.0),
                font: PdfFontHandle::External(header_font.clone()),
            },
            Op::ShowText {
                items: vec![TextItem::Text(header_text)],
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

    /// Ensures at least `needed_pt` of vertical space remains on the current page, breaking if needed.
    pub fn ensure_space(&mut self, needed_pt: f32) {
        self.flush_break();
        if self.remaining() < needed_pt {
            self.start_new_page();
        }
    }

    /// Width in points available for text between the two margins.
    pub fn usable_width_pt(&self) -> f32 {
        self.page_width.into_pt().0 - 2.0 * self.margin.into_pt().0
    }

    /// The line height in points used by this builder.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Remaining vertical space in points on the current page.
    pub fn remaining_pt(&self) -> f32 {
        self.usable_height() - self.y
    }

    /// Emits an invisible link annotation covering the last `height_pt` of vertical space written.
    ///
    /// Must be called immediately after the text it should cover (e.g. `write_line*` or
    /// `write_centered`). Pass `builder.line_height()` for a single standard line, or
    /// `n as f32 * builder.line_height()` for n lines (used in TOC). For non-standard font
    /// sizes (e.g. the cover title at 28 pt) pass `font_size + leading` directly.
    ///
    /// The ascender shift is clamped to one line height so multi-row spans don't
    /// shift the entire rect up by their full height.
    pub fn add_link(&mut self, height_pt: f32, action: Actions) {
        // In printpdf, text is placed at its baseline. Visual glyphs extend
        // ~0.7× above (ascenders) and ~0.2× below (descenders) a single line.
        // Shift up by 0.8× of one line so the rect covers what users see.
        let ascender_shift = height_pt.min(self.line_height) * 0.8;
        let y_bottom = Pt(
            self.page_height.into_pt().0 - self.margin.into_pt().0 - 12.0 - self.y + ascender_shift,
        );
        let rect = Rect::from_xywh(
            self.left_x(),
            y_bottom,
            Pt(self.usable_width_pt()),
            Pt(height_pt),
        );
        self.current_ops.push(Op::LinkAnnotation {
            link: LinkAnnotation::new(
                rect,
                action,
                Some(BorderArray::Solid([0.0, 0.0, 0.0])),
                Some(ColorArray::Transparent),
                None,
            ),
        });
    }

    /// Mark a section boundary. The new page is created lazily on the next write,
    /// so finish() never produces a trailing empty page.
    pub fn page_break(&mut self) {
        self.pending_break = true;
    }

    /// Writes a line of styled spans left-aligned at the current cursor position.
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
                Op::SetFont {
                    size: span.size,
                    font: PdfFontHandle::External(span.font_id.clone()),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(span.text.clone())],
                },
            ]
        }));

        self.current_ops.push(Op::EndTextSection);
        self.y += self.line_height;
    }

    /// Advances the cursor downward by `pt` points without writing any content.
    pub fn vertical_space(&mut self, pt: f32) {
        self.y += pt;
    }

    /// Writes a single string centered horizontally on the current line.
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
            Op::SetFont {
                size,
                font: PdfFontHandle::External(font_id.clone()),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ]);

        self.y += size.0 + 4.0;
    }

    /// Writes a line of styled spans centered horizontally on the page.
    pub fn write_line_centered(&mut self, spans: &[Span]) {
        self.ensure_space(self.line_height);
        let y = self.pdf_y();

        let total_width: f32 = spans
            .iter()
            .map(|s| s.text.len() as f32 * s.size.0 * 0.6)
            .sum();
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
                Op::SetFont {
                    size: span.size,
                    font: PdfFontHandle::External(span.font_id.clone()),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(span.text.clone())],
                },
            ]
        }));
        self.current_ops.push(Op::EndTextSection);
        self.y += self.line_height;
    }

    /// Writes two groups of spans: `left` aligned to the left margin and `right` to the right margin.
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
                Op::SetFont {
                    size: span.size,
                    font: PdfFontHandle::External(span.font_id.clone()),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(span.text.clone())],
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
                Op::SetFont {
                    size: span.size,
                    font: PdfFontHandle::External(span.font_id.clone()),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(span.text.clone())],
                },
            ]
        }));
        self.current_ops.push(Op::EndTextSection);

        self.y += self.line_height;
    }

    /// Draw a full-width horizontal rule at the current `y` position and advance
    /// `y` by `thickness_pt` so subsequent content clears the rule.
    pub fn draw_horizontal_rule(&mut self, color: Color, thickness_pt: f32) {
        self.flush_break();
        let y = self.pdf_y();
        let left = self.left_x();
        let right = Pt(left.0 + self.usable_width_pt());
        self.current_ops.extend([
            Op::SaveGraphicsState,
            Op::SetOutlineColor { col: color },
            Op::SetOutlineThickness {
                pt: Pt(thickness_pt),
            },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point { x: left, y },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point { x: right, y },
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
            Op::RestoreGraphicsState,
        ]);
        self.y += thickness_pt;
    }

    /// Draw a filled rectangle.
    ///
    /// - `x_offset_pt`: x position from the left margin.
    /// - `y_below_cursor_pt`: distance below the current cursor to the **bottom** edge of the rect.
    /// - `width_pt`, `height_pt`: dimensions (rect grows upward from the bottom edge).
    ///
    /// Does **not** advance `y` — call `vertical_space` afterward if needed.
    pub fn draw_filled_rect(
        &mut self,
        x_offset_pt: f32,
        y_below_cursor_pt: f32,
        width_pt: f32,
        height_pt: f32,
        color: Color,
    ) {
        self.flush_break();
        let x = self.left_x().0 + x_offset_pt;
        let y_bottom = self.pdf_y().0 - y_below_cursor_pt;
        let lp = |px: f32, py: f32| LinePoint {
            p: Point {
                x: Pt(px),
                y: Pt(py),
            },
            bezier: false,
        };
        let polygon = Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    lp(x, y_bottom),
                    lp(x + width_pt, y_bottom),
                    lp(x + width_pt, y_bottom + height_pt),
                    lp(x, y_bottom + height_pt),
                ],
            }],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        };
        self.current_ops.extend([
            Op::SaveGraphicsState,
            Op::SetFillColor { col: color },
            Op::DrawPolygon { polygon },
            Op::RestoreGraphicsState,
        ]);
    }

    /// Write text at a specific x offset from the left margin, at the current `y` cursor.
    /// Does **not** advance `y`.
    pub fn write_text_at_x(
        &mut self,
        x_offset_pt: f32,
        text: &str,
        font_id: &FontId,
        size: Pt,
        color: Color,
    ) {
        self.flush_break();
        let x = Pt(self.left_x().0 + x_offset_pt);
        self.current_ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point { x, y: self.pdf_y() },
            },
            Op::SetFillColor { col: color },
            Op::SetFont {
                size,
                font: PdfFontHandle::External(font_id.clone()),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ]);
    }

    /// Returns the appropriate `FontId` for the requested bold/italic combination.
    pub fn font(&self, bold: bool, italic: bool) -> &FontId {
        match (bold, italic) {
            (true, true) => &self.fonts.bold_italic,
            (true, false) => &self.fonts.bold,
            (false, true) => &self.fonts.italic,
            (false, false) => &self.fonts.regular,
        }
    }

    /// Finalizes all pages and returns them; no trailing empty page is produced.
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
    fn draw_horizontal_rule_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        builder.draw_horizontal_rule(Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)), 0.5);
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

    #[test]
    fn write_line_centered_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line_centered(&[Span {
            text: "centered".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn write_line_justified_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line_justified(
            &[Span {
                text: "left".into(),
                font_id: fonts.regular.clone(),
                size: Pt(8.0),
                color: black(),
            }],
            &[Span {
                text: "right".into(),
                font_id: fonts.bold.clone(),
                size: Pt(8.0),
                color: black(),
            }],
        );
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn draw_filled_rect_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        builder.draw_filled_rect(0.0, 20.0, 100.0, 10.0, black());
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn write_text_at_x_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_text_at_x(50.0, "hello", &fonts.regular, Pt(8.0), black());
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn font_variants_are_distinct() {
        let (_doc, fonts) = test_font_set();
        let builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        // Each combination must return without panic; IDs may or may not be equal.
        let _ = builder.font(false, false);
        let _ = builder.font(true, false);
        let _ = builder.font(false, true);
        let _ = builder.font(true, true);
    }

    #[test]
    fn usable_width_pt_is_positive() {
        let (_doc, fonts) = test_font_set();
        let builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        assert!(builder.usable_width_pt() > 0.0);
    }

    #[test]
    fn line_height_matches_constructor() {
        let (_doc, fonts) = test_font_set();
        let builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 12.5, fonts, 1);
        assert_eq!(builder.line_height(), 12.5);
    }

    #[test]
    fn remaining_pt_decreases_after_write() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        let before = builder.remaining_pt();
        builder.write_line(&[Span {
            text: "x".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        assert!(builder.remaining_pt() < before);
    }

    #[test]
    fn current_page_with_pending_break() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line(&[Span {
            text: "x".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        let page_before = builder.current_page();
        builder.page_break();
        // current_page() should report the upcoming page while break is pending.
        assert_eq!(builder.current_page(), page_before + 1);
    }

    #[test]
    fn vertical_space_reduces_remaining() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        let before = builder.remaining_pt();
        builder.vertical_space(20.0);
        assert!((builder.remaining_pt() - (before - 20.0)).abs() < 0.01);
    }

    #[test]
    fn ensure_space_forces_page_break_when_tight() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 1);
        // Consume almost all space, then request more than what remains.
        let usable = builder.remaining_pt();
        builder.vertical_space(usable - 5.0);
        let page_before = builder.current_page();
        builder.ensure_space(50.0);
        assert!(builder.current_page() > page_before);
    }

    #[test]
    fn add_link_does_not_panic() {
        let (_doc, fonts) = test_font_set();
        let mut builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts.clone(), 1);
        builder.write_line(&[Span {
            text: "linked text".into(),
            font_id: fonts.regular.clone(),
            size: Pt(8.0),
            color: black(),
        }]);
        builder.add_link(
            10.0,
            printpdf::Actions::Uri("https://example.com".to_string()),
        );
        assert_eq!(builder.finish().len(), 1);
    }

    #[test]
    fn starting_page_offset_is_respected() {
        let (_doc, fonts) = test_font_set();
        let builder = PageBuilder::new(Mm(210.0), Mm(297.0), Mm(10.0), 10.0, fonts, 5);
        assert_eq!(builder.current_page(), 5);
    }
}
