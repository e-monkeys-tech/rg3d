use crate::{
    brush::Brush,
    core::{algebra::Vector2, color::Color, math::Rect},
    ttf::SharedFont,
    HorizontalAlignment, VerticalAlignment,
};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct TextGlyph {
    bounds: Rect<f32>,
    tex_coords: [Vector2<f32>; 4],
}

impl TextGlyph {
    pub fn get_bounds(&self) -> Rect<f32> {
        self.bounds
    }

    pub fn get_tex_coords(&self) -> &[Vector2<f32>; 4] {
        &self.tex_coords
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextLine {
    /// Index of starting symbol in text array.
    pub begin: usize,
    /// Index of ending symbol in text array.
    pub end: usize,
    /// Total width of line.
    pub width: f32,
    /// Total height of line. Usually just ascender of a font.
    pub height: f32,
    /// Local horizontal position of line.
    pub x_offset: f32,
    /// Local vertical position of line.
    pub y_offset: f32,
}

impl TextLine {
    fn new() -> TextLine {
        TextLine {
            begin: 0,
            end: 0,
            width: 0.0,
            height: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.begin
    }
}

/// Wrapping mode for formatted text.
#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
pub enum WrapMode {
    /// No wrapping needed.
    NoWrap,

    /// Letter-based wrapping.
    Letter,

    /// Word-based wrapping.
    Word,
}

#[derive(Clone, Debug)]
pub struct FormattedText {
    font: Option<SharedFont>,
    /// Text in UTF32 format.
    text: Vec<u32>,
    /// Temporary buffer used to split text on lines. We need it to reduce memory allocations
    /// when we changing text too frequently, here we sacrifice some memory in order to get
    /// more performance.
    lines: Vec<TextLine>,
    /// Final glyphs for draw buffer.
    glyphs: Vec<TextGlyph>,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    brush: Brush,
    constraint: Vector2<f32>,
    wrap: WrapMode,
    mask_char: Option<char>,
}

#[derive(Copy, Clone, Debug)]
struct Word {
    width: f32,
    length: usize,
}

impl FormattedText {
    pub fn get_glyphs(&self) -> &[TextGlyph] {
        &self.glyphs
    }

    pub fn get_font(&self) -> Option<SharedFont> {
        self.font.clone()
    }

    pub fn set_font(&mut self, font: SharedFont) -> &mut Self {
        self.font = Some(font);
        self
    }

    pub fn get_lines(&self) -> &[TextLine] {
        &self.lines
    }

    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) -> &mut Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }

    pub fn set_horizontal_alignment(
        &mut self,
        horizontal_alignment: HorizontalAlignment,
    ) -> &mut Self {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }

    pub fn set_brush(&mut self, brush: Brush) -> &mut Self {
        self.brush = brush;
        self
    }

    pub fn brush(&self) -> Brush {
        self.brush.clone()
    }

    pub fn set_constraint(&mut self, constraint: Vector2<f32>) -> &mut Self {
        self.constraint = constraint;
        self
    }

    pub fn get_raw_text(&self) -> &[u32] {
        &self.text
    }

    pub fn text(&self) -> String {
        self.text
            .iter()
            .filter_map(|c| std::char::from_u32(*c))
            .collect()
    }

    pub fn get_range_width<T: IntoIterator<Item = usize>>(&self, range: T) -> f32 {
        let mut width = 0.0;
        if let Some(ref font) = self.font {
            let font = font.0.lock().unwrap();
            for index in range {
                width += font.glyph_advance(self.text[index]);
            }
        }
        width
    }

    pub fn set_text<P: AsRef<str>>(&mut self, text: P) -> &mut Self {
        // Convert text to UTF32.
        self.text.clear();
        for code in text.as_ref().chars().map(|c| c as u32) {
            self.text.push(code);
        }
        self
    }

    pub fn set_wrap(&mut self, wrap: WrapMode) -> &mut Self {
        self.wrap = wrap;
        self
    }

    pub fn wrap_mode(&self) -> WrapMode {
        self.wrap
    }

    pub fn insert_char(&mut self, c: char, index: usize) -> &mut Self {
        self.text.insert(index, c as u32);
        self
    }

    pub fn insert_str(&mut self, str: &str, position: usize) -> &mut Self {
        for (i, char) in str.chars().enumerate() {
            self.text.insert(position + i, char as u32);
        }
        self
    }

    pub fn remove_range(&mut self, range: Range<usize>) -> &mut Self {
        self.text.drain(range);
        self
    }

    pub fn remove_at(&mut self, index: usize) -> &mut Self {
        self.text.remove(index);
        self
    }

    pub fn build(&mut self) -> Vector2<f32> {
        let font = if let Some(font) = &self.font {
            font.0.lock().unwrap()
        } else {
            return Vector2::default();
        };

        let masked_text: Vec<u32>;
        let text: &Vec<u32> = if let Some(mask_char) = self.mask_char {
            masked_text = (0..self.text.len()).map(|_| mask_char as u32).collect();
            &masked_text
        } else {
            &self.text
        };

        // Split on lines.
        let mut total_height = 0.0;
        let mut current_line = TextLine::new();
        let mut word: Option<Word> = None;
        self.lines.clear();
        for (i, code) in text.iter().enumerate() {
            let advance = match font.glyph(*code) {
                Some(glyph) => glyph.advance,
                None => font.height(),
            };
            let is_new_line = *code == u32::from(b'\n') || *code == u32::from(b'\r');
            let new_width = current_line.width + advance;
            let is_white_space = char::from_u32(*code).map_or(false, |c| c.is_whitespace());
            let word_ended = word.is_some() && is_white_space || i == text.len() - 1;

            if self.wrap == WrapMode::Word && !is_white_space {
                match word.as_mut() {
                    Some(word) => {
                        word.width += advance;
                        word.length += 1;
                    }
                    None => {
                        word = Some(Word {
                            width: advance,
                            length: 1,
                        });
                    }
                };
            }

            if is_new_line {
                if let Some(word) = word.take() {
                    current_line.width += word.width;
                    current_line.end += word.length;
                }
                self.lines.push(current_line);
                current_line.begin = if is_new_line { i + 1 } else { i };
                current_line.end = current_line.begin;
                current_line.width = advance;
                total_height += font.ascender();
            } else {
                match self.wrap {
                    WrapMode::NoWrap => {
                        current_line.width = new_width;
                        current_line.end += 1;
                    }
                    WrapMode::Letter => {
                        if new_width > self.constraint.x {
                            self.lines.push(current_line);
                            current_line.begin = if is_new_line { i + 1 } else { i };
                            current_line.end = current_line.begin + 1;
                            current_line.width = advance;
                            total_height += font.ascender();
                        } else {
                            current_line.width = new_width;
                            current_line.end += 1;
                        }
                    }
                    WrapMode::Word => {
                        if word_ended {
                            let word = word.take().unwrap();
                            if word.width > self.constraint.x {
                                // The word is longer than available constraints.
                                // Push the word as a whole.
                                current_line.width += word.width;
                                current_line.end += word.length;
                                self.lines.push(current_line);
                                current_line.begin = current_line.end;
                                current_line.width = 0.0;
                                total_height += font.ascender();
                            } else if current_line.width + word.width > self.constraint.x {
                                // The word will exceed horizontal constraint, we have to
                                // commit current line and move the word in the next line.
                                self.lines.push(current_line);
                                current_line.begin = i - word.length;
                                current_line.end = i;
                                current_line.width = word.width;
                                total_height += font.ascender();
                            } else {
                                // The word does not exceed horizontal constraint, append it
                                // to the line.
                                current_line.width += word.width;
                                current_line.end += word.length;
                            }
                        }

                        // White-space characters are not part of word so pass them through.
                        if is_white_space {
                            current_line.end += 1;
                            current_line.width += advance;
                        }
                    }
                }
            }
        }
        // Commit rest of text.
        if current_line.begin != current_line.end {
            for code in text.iter().skip(current_line.end) {
                let advance = match font.glyph(*code) {
                    Some(glyph) => glyph.advance,
                    None => font.height(),
                };
                current_line.width += advance;
            }
            current_line.end = text.len();
            self.lines.push(current_line);
            total_height += font.ascender();
        }

        // Align lines according to desired alignment.
        for line in self.lines.iter_mut() {
            match self.horizontal_alignment {
                HorizontalAlignment::Left => line.x_offset = 0.0,
                HorizontalAlignment::Center => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = 0.0;
                    } else {
                        line.x_offset = 0.5 * (self.constraint.x - line.width).max(0.0);
                    }
                }
                HorizontalAlignment::Right => {
                    if self.constraint.x.is_infinite() {
                        line.x_offset = 0.0;
                    } else {
                        line.x_offset = (self.constraint.x - line.width).max(0.0)
                    }
                }
                HorizontalAlignment::Stretch => line.x_offset = 0.0,
            }
        }

        // Generate glyphs for each text line.
        self.glyphs.clear();

        let cursor_y_start = match self.vertical_alignment {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Center => {
                if self.constraint.y.is_infinite() {
                    0.0
                } else {
                    (self.constraint.y - total_height).max(0.0) * 0.5
                }
            }
            VerticalAlignment::Bottom => {
                if self.constraint.y.is_infinite() {
                    0.0
                } else {
                    (self.constraint.y - total_height).max(0.0)
                }
            }
            VerticalAlignment::Stretch => 0.0,
        };

        let cursor_x_start = if self.constraint.x.is_infinite() {
            0.0
        } else {
            self.constraint.x
        };

        let mut cursor = Vector2::new(cursor_x_start, cursor_y_start);
        for line in self.lines.iter_mut() {
            cursor.x = line.x_offset;

            for &code in text.iter().take(line.end).skip(line.begin) {
                match font.glyph(code) {
                    Some(glyph) => {
                        // Insert glyph
                        let rect = Rect::new(
                            cursor.x + glyph.left.floor(),
                            cursor.y + font.ascender().floor()
                                - glyph.top.floor()
                                - glyph.bitmap_height as f32,
                            glyph.bitmap_width as f32,
                            glyph.bitmap_height as f32,
                        );
                        let text_glyph = TextGlyph {
                            bounds: rect,
                            tex_coords: glyph.tex_coords,
                        };
                        self.glyphs.push(text_glyph);

                        cursor.x += glyph.advance;
                    }
                    None => {
                        // Insert invalid symbol
                        let rect = Rect::new(
                            cursor.x,
                            cursor.y + font.ascender(),
                            font.height(),
                            font.height(),
                        );
                        self.glyphs.push(TextGlyph {
                            bounds: rect,
                            tex_coords: [Vector2::default(); 4],
                        });
                        cursor.x += rect.w();
                    }
                }
            }
            line.height = font.ascender();
            line.y_offset = cursor.y;
            cursor.y += font.ascender();
        }

        // Minus here is because descender has negative value.
        let mut full_size = Vector2::new(0.0, total_height - font.descender());
        for line in self.lines.iter() {
            full_size.x = line.width.max(full_size.x);
        }
        full_size
    }
}

pub struct FormattedTextBuilder {
    font: Option<SharedFont>,
    brush: Brush,
    constraint: Vector2<f32>,
    text: String,
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
    wrap: WrapMode,
    mask_char: Option<char>,
}

impl Default for FormattedTextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FormattedTextBuilder {
    /// Creates new formatted text builder with default parameters.
    pub fn new() -> FormattedTextBuilder {
        FormattedTextBuilder {
            font: None,
            text: "".to_owned(),
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            brush: Brush::Solid(Color::WHITE),
            constraint: Vector2::new(128.0, 128.0),
            wrap: WrapMode::NoWrap,
            mask_char: None,
        }
    }

    pub fn with_font(mut self, font: SharedFont) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_vertical_alignment(mut self, vertical_alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = vertical_alignment;
        self
    }

    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn with_horizontal_alignment(mut self, horizontal_alignment: HorizontalAlignment) -> Self {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    pub fn with_constraint(mut self, constraint: Vector2<f32>) -> Self {
        self.constraint = constraint;
        self
    }

    pub fn with_brush(mut self, brush: Brush) -> Self {
        self.brush = brush;
        self
    }

    pub fn with_mask_char(mut self, mask_char: Option<char>) -> Self {
        self.mask_char = mask_char;
        self
    }

    pub fn build(self) -> FormattedText {
        FormattedText {
            font: self.font,
            text: self.text.chars().map(|c| c as u32).collect(),
            lines: Vec::new(),
            glyphs: Vec::new(),
            vertical_alignment: self.vertical_alignment,
            horizontal_alignment: self.horizontal_alignment,
            brush: self.brush,
            constraint: self.constraint,
            wrap: self.wrap,
            mask_char: self.mask_char,
        }
    }
}
