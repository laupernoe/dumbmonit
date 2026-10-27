//! Un écrivain PDF minimal, sans dépendance : texte (Helvetica et Helvetica
//! gras, polices standard que tout lecteur PDF fournit), rectangles et filets.
//!
//! Pourquoi pas une bibliothèque : le rapport est un document de texte et de
//! tableaux. Les polices standard dispensent d'en embarquer une ; le format PDF
//! pour ce cas tient en un catalogue, un arbre de pages, deux polices et un flux
//! de contenu par page. Une dépendance de plus (et son arbre de licences à faire
//! passer par `cargo deny`) ne ferait rien de mieux, et le binaire reste
//! reproductible octet pour octet : aucune date ni identifiant aléatoire n'est
//! écrit, sinon celui que l'appelant fournit.
//!
//! Texte en WinAnsi (Windows-1252) : le latin accentué passe, le reste est
//! remplacé par `?`. Les rapports sont en anglais, les noms d'équipements
//! viennent de l'utilisateur.

use std::fmt::Write as _;

/// Page A4 en points (1/72 de pouce).
pub const PAGE_W: f64 = 595.0;
pub const PAGE_H: f64 = 842.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Regular,
    Bold,
}

impl Font {
    fn resource(self) -> &'static str {
        match self {
            Self::Regular => "F1",
            Self::Bold => "F2",
        }
    }
}

/// Couleur RVB, composantes de 0 à 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn ops(self) -> String {
        let part = |value: u8| f64::from(value) / 255.0;
        format!("{:.3} {:.3} {:.3}", part(self.0), part(self.1), part(self.2))
    }
}

/// Largeurs Helvetica (en 1/1000 d'em) des caractères ASCII imprimables 32..=126.
const REGULAR: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Même table pour Helvetica gras.
const BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// Un caractère en octet WinAnsi.
fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        // Latin-1 : mêmes valeurs en Windows-1252.
        '\u{a0}'..='\u{ff}' => c as u32 as u8,
        '€' => 0x80,
        '…' => 0x85,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        _ => b'?',
    }
}

/// Largeur d'un texte en points, à la taille `size`.
pub fn text_width(text: &str, font: Font, size: f64) -> f64 {
    let table = if font == Font::Bold { &BOLD } else { &REGULAR };
    let units: u32 = text
        .chars()
        .map(|c| match win_ansi(c) {
            b @ 32..=126 => u32::from(table[usize::from(b - 32)]),
            _ => 556,
        })
        .sum();
    f64::from(units) * size / 1000.0
}

/// Coupe le texte pour qu'il tienne en `max` points, en finissant par « ... ».
pub fn fit(text: &str, font: Font, size: f64, max: f64) -> String {
    if text_width(text, font, size) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        out.push(c);
        if text_width(&format!("{out}..."), font, size) > max {
            out.pop();
            break;
        }
    }
    format!("{}...", out.trim_end())
}

/// Chaîne littérale PDF : parenthèses, barres obliques et octets hors ASCII échappés.
fn literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('(');
    for c in text.chars() {
        match win_ansi(c) {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            b @ 32..=126 => out.push(b as char),
            // Octets hauts en octal : le fichier reste de l'ASCII pur.
            b => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push(')');
    out
}

/// Une page en construction : un flux d'opérateurs de dessin.
#[derive(Default)]
pub struct Page {
    ops: String,
}

impl Page {
    /// Texte dont la ligne de base part de (x, y), l'origine étant en bas à gauche.
    pub fn text(&mut self, x: f64, y: f64, text: &str, font: Font, size: f64, color: Rgb) {
        let _ = writeln!(
            self.ops,
            "BT {} rg /{} {size:.1} Tf {x:.2} {y:.2} Td {} Tj ET",
            color.ops(),
            font.resource(),
            literal(text)
        );
    }

    /// Texte aligné à droite sur `right`.
    pub fn text_right(
        &mut self,
        right: f64,
        y: f64,
        text: &str,
        font: Font,
        size: f64,
        color: Rgb,
    ) {
        let x = right - text_width(text, font, size);
        self.text(x, y, text, font, size, color);
    }

    pub fn rect_fill(&mut self, x: f64, y: f64, w: f64, h: f64, color: Rgb) {
        let _ = writeln!(self.ops, "{} rg {x:.2} {y:.2} {w:.2} {h:.2} re f", color.ops());
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, width: f64, color: Rgb) {
        let _ = writeln!(
            self.ops,
            "{} RG {width:.2} w {x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S",
            color.ops()
        );
    }

    /// Dessine l'image n° `index` du document (voir [`Document::add_image`])
    /// dans le rectangle (x, y, w, h), coin bas gauche en (x, y).
    pub fn image(&mut self, index: usize, x: f64, y: f64, w: f64, h: f64) {
        let _ = writeln!(self.ops, "q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /Im{index} Do Q");
    }
}

/// Une image matricielle prête à incorporer : JPEG tel quel (`DCTDecode`), ou
/// pixels compressés (`FlateDecode`) avec un masque alpha facultatif. Les
/// octets sont écrits en hexadécimal (`ASCIIHexDecode`) : le fichier reste de
/// l'ASCII pur, comme le reste du document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `DeviceRGB` ou `DeviceGray`.
    pub color_space: &'static str,
    /// `DCTDecode` (JPEG) ou `FlateDecode` (pixels compressés par zlib).
    pub filter: &'static str,
    pub data: Vec<u8>,
    /// Masque alpha, pixels gris de 8 bits compressés par zlib.
    pub alpha: Option<Vec<u8>>,
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out.push('>');
    out
}

/// Le document : des pages, un titre et une date de création optionnelle.
pub struct Document {
    pages: Vec<Page>,
    images: Vec<Image>,
    title: String,
    /// `D:AAAAMMJJHHMMSSZ`, écrit tel quel dans les métadonnées.
    created: Option<String>,
}

impl Document {
    pub fn new(title: &str, created: Option<String>) -> Self {
        Self { pages: Vec::new(), images: Vec::new(), title: title.to_string(), created }
    }

    /// Ajoute une image ; rend l'index à passer à [`Page::image`].
    pub fn add_image(&mut self, image: Image) -> usize {
        self.images.push(image);
        self.images.len() - 1
    }

    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Le fichier PDF complet.
    pub fn finish(self) -> Vec<u8> {
        let n = self.pages.len().max(1);
        // Numéros d'objets : 1 catalogue, 2 pages, 3-4 polices, 5 métadonnées,
        // puis (page, flux) par page.
        let first_page = 6;
        let mut objects: Vec<String> = Vec::new();
        objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
        let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", first_page + 2 * i)).collect();
        objects.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")));
        objects.push(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_string(),
        );
        objects.push(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
                .to_string(),
        );
        let mut info = format!("<< /Title {} /Producer (DumbMonit)", literal(&self.title));
        if let Some(created) = &self.created {
            let _ = write!(info, " /CreationDate ({created})");
        }
        info.push_str(" >>");
        objects.push(info);

        let empty = Page::default();
        let pages: Vec<&Page> =
            if self.pages.is_empty() { vec![&empty] } else { self.pages.iter().collect() };
        // Les images suivent les pages : chacune prend un objet, plus un pour
        // son masque alpha.
        let mut next = first_page + 2 * n;
        let image_objects: Vec<(usize, Option<usize>)> = self
            .images
            .iter()
            .map(|image| {
                let main = next;
                let mask = image.alpha.as_ref().map(|_| main + 1);
                next += 1 + usize::from(mask.is_some());
                (main, mask)
            })
            .collect();
        let xobjects = if image_objects.is_empty() {
            String::new()
        } else {
            let refs: Vec<String> = image_objects
                .iter()
                .enumerate()
                .map(|(i, (main, _))| format!("/Im{i} {main} 0 R"))
                .collect();
            format!(" /XObject << {} >>", refs.join(" "))
        };
        for (i, page) in pages.iter().enumerate() {
            let stream = first_page + 2 * i + 1;
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >>{xobjects} >> \
                 /Contents {stream} 0 R >>"
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}endstream",
                page.ops.len(),
                page.ops
            ));
        }
        for (image, (_, mask)) in self.images.iter().zip(&image_objects) {
            let data = hex(&image.data);
            let smask = mask.map(|m| format!(" /SMask {m} 0 R")).unwrap_or_default();
            objects.push(format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /{} \
                 /BitsPerComponent 8 /Filter [/ASCIIHexDecode /{}]{smask} /Length {} >>\n\
                 stream\n{data}\nendstream",
                image.width,
                image.height,
                image.color_space,
                image.filter,
                data.len()
            ));
            if let Some(alpha) = &image.alpha {
                let data = hex(alpha);
                objects.push(format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} \
                     /ColorSpace /DeviceGray /BitsPerComponent 8 \
                     /Filter [/ASCIIHexDecode /FlateDecode] /Length {} >>\n\
                     stream\n{data}\nendstream",
                    image.width,
                    image.height,
                    data.len()
                ));
            }
        }

        // Tout est de l'ASCII : les octets hauts du texte sont écrits en octal.
        let mut out = String::from("%PDF-1.4\n");
        let mut offsets = Vec::with_capacity(objects.len());
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
        }
        let xref = out.len();
        let count = objects.len() + 1;
        let _ = write!(out, "xref\n0 {count}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {count} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        );
        out.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Rgb = Rgb(0, 0, 0);

    fn sample() -> Vec<u8> {
        let mut page = Page::default();
        page.text(40.0, 800.0, "Availability (report) \\ é €", Font::Bold, 12.0, BLACK);
        page.rect_fill(40.0, 700.0, 100.0, 20.0, Rgb(200, 220, 255));
        page.line(40.0, 690.0, 500.0, 690.0, 0.5, BLACK);
        let mut doc = Document::new("Acme report", Some("D:20261101000000Z".into()));
        doc.add_page(page);
        doc.add_page(Page::default());
        doc.finish()
    }

    #[test]
    fn the_file_has_the_pdf_framing() {
        let pdf = sample();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("/Count 2"));
        assert!(text.contains("/BaseFont /Helvetica-Bold"));
        assert!(text.contains("/Title (Acme report)"));
        assert!(text.contains("/CreationDate (D:20261101000000Z)"));
    }

    #[test]
    fn the_xref_offsets_point_at_the_objects() {
        let pdf = sample();
        let text = String::from_utf8_lossy(&pdf).into_owned();
        let start: usize = text
            .rsplit("startxref\n")
            .next()
            .and_then(|tail| tail.lines().next())
            .and_then(|n| n.parse().ok())
            .expect("startxref");
        assert!(text[start..].starts_with("xref\n"), "startxref points at the table");
        let table: Vec<&str> =
            text[start..].lines().skip(3).take_while(|l| l.ends_with(" n ")).collect();
        assert!(table.len() >= 9);
        for (i, entry) in table.iter().enumerate() {
            let offset: usize = entry[..10].parse().expect("offset");
            assert!(
                text[offset..].starts_with(&format!("{} 0 obj", i + 1)),
                "object {} is not at {offset}",
                i + 1
            );
        }
    }

    #[test]
    fn the_content_stream_length_is_exact() {
        let pdf = sample();
        let text = String::from_utf8_lossy(&pdf).into_owned();
        let at = text.find("<< /Length ").expect("a stream");
        let rest = &text[at + "<< /Length ".len()..];
        let declared: usize = rest.split(' ').next().unwrap().parse().unwrap();
        let body_start = rest.find("stream\n").unwrap() + "stream\n".len();
        let body_end = rest.find("endstream").unwrap();
        assert_eq!(declared, body_end - body_start);
    }

    #[test]
    fn an_image_is_referenced_by_every_page_and_stays_ascii() {
        let mut doc = Document::new("Logo", None);
        let index = doc.add_image(Image {
            width: 1,
            height: 1,
            color_space: "DeviceGray",
            filter: "FlateDecode",
            data: vec![0x78, 0x9c, 0xff],
            alpha: Some(vec![0x00, 0x01]),
        });
        let mut page = Page::default();
        page.image(index, 10.0, 10.0, 20.0, 20.0);
        doc.add_page(page);
        doc.add_page(Page::default());
        let pdf = doc.finish();
        assert!(pdf.is_ascii());
        let text = String::from_utf8_lossy(&pdf);
        assert_eq!(text.matches("/XObject << /Im0 10 0 R >>").count(), 2, "{text}");
        assert!(text.contains("/Im0 Do"));
        assert!(text.contains("/SMask 11 0 R"));
        assert!(text.contains("789cff>"));
        assert!(text.contains("0001>"));
    }

    #[test]
    fn special_characters_are_escaped_and_encoded() {
        assert_eq!(literal("a(b)c\\d"), "(a\\(b\\)c\\\\d)");
        // é est 0xE9 (351 en octal), € est 0x80 (200), un idéogramme devient « ? ».
        assert_eq!(literal("é€漢"), "(\\351\\200?)");
        let pdf = sample();
        assert!(pdf.is_ascii(), "pure ASCII, so byte offsets equal string offsets");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("\\351 \\200"), "latin text is octal-escaped in the stream");
    }

    #[test]
    fn text_is_measured_and_truncated_to_fit() {
        let w = text_width("Hello", Font::Regular, 10.0);
        assert!((w - (722.0 + 556.0 + 222.0 + 222.0 + 556.0) * 10.0 / 1000.0).abs() < 1e-9);
        assert!(text_width("Hello", Font::Bold, 10.0) > w);
        let cut = fit("a very long monitor name that will not fit", Font::Regular, 9.0, 80.0);
        assert!(cut.ends_with("...") && text_width(&cut, Font::Regular, 9.0) <= 80.0, "{cut}");
        assert_eq!(fit("short", Font::Regular, 9.0, 80.0), "short");
    }
}
