//! Text encoding and line-ending detection/conversion. The editor always holds `\n`-separated Unicode;
//! this module maps that to and from the bytes on disk.

use std::borrow::Cow;
use std::io;

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
            Self::Cr => "CR",
        }
    }

    fn separator(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// How a file is (or will be) stored on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub encoding: &'static Encoding,
    pub bom: bool,
    pub eol: LineEnding,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            encoding: UTF_8,
            bom: false,
            eol: LineEnding::Lf,
        }
    }
}

impl Format {
    /// e.g. "UTF-8", "UTF-8 BOM", "windows-1252".
    pub fn encoding_label(&self) -> String {
        if self.bom {
            format!("{} BOM", self.encoding.name())
        } else {
            self.encoding.name().to_string()
        }
    }
}

/// The most frequent line ending (ties prefer LF, then CRLF). Known limitation: a file mixing endings is
/// reported and, when not converting, saved with the dominant one only.
fn detect_eol(text: &str) -> LineEnding {
    let crlf = text.matches("\r\n").count();
    let lf = text.matches('\n').count() - crlf;
    let cr = text.matches('\r').count() - crlf;
    if lf >= crlf && lf >= cr {
        LineEnding::Lf
    } else if crlf >= cr {
        LineEnding::CrLf
    } else {
        LineEnding::Cr
    }
}

/// Without a BOM: the `hint` (the encoding the document already has) if it decodes cleanly, else UTF-8 if valid,
/// else a statistical guess.
fn pick(bytes: &[u8], hint: Option<&'static Encoding>) -> &'static Encoding {
    if let Some(h) = hint {
        if h.decode_without_bom_handling_and_without_replacement(bytes)
            .is_some()
        {
            return h;
        }
    }
    if std::str::from_utf8(bytes).is_ok() {
        return UTF_8;
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    detector.guess(None, true)
}

/// Decode file bytes to `\n`-normalised text plus the format they were stored in. Never fails: malformed
/// input is replaced with U+FFFD. `hint` keeps a re-read of a known file from being re-guessed differently.
pub fn decode(bytes: &[u8], hint: Option<&'static Encoding>) -> (String, Format) {
    let (encoding, bom, body) = match Encoding::for_bom(bytes) {
        Some((e, n)) => (e, true, bytes.get(n..).unwrap_or_default()),
        None => (pick(bytes, hint), false, bytes),
    };
    let (text, _) = encoding.decode_without_bom_handling(body);
    let eol = detect_eol(&text);
    let text = match eol {
        LineEnding::Lf => text.into_owned(),
        _ => text.replace("\r\n", "\n").replace('\r', "\n"),
    };
    crate::trace!(
        "decode: {} bytes as {} (bom {bom}), line ending {}",
        bytes.len(),
        encoding.name(),
        eol.label()
    );
    (text, Format { encoding, bom, eol })
}

/// Encode `\n`-separated text for disk. A converting option targets UTF-8 / LF, otherwise the file's own
/// format is kept. Returns the bytes and the format they use. Fails (nothing to write) if the text has
/// characters the kept encoding can't represent.
pub fn encode(
    text: &str,
    current: Format,
    convert_encoding: bool,
    convert_eol: bool,
) -> io::Result<(Vec<u8>, Format)> {
    let target = Format {
        encoding: if convert_encoding {
            UTF_8
        } else {
            current.encoding
        },
        bom: !convert_encoding && current.bom,
        eol: if convert_eol {
            LineEnding::Lf
        } else {
            current.eol
        },
    };
    let body = match target.eol {
        LineEnding::Lf => Cow::Borrowed(text),
        eol => Cow::Owned(text.replace('\n', eol.separator())),
    };
    let mut out = Vec::with_capacity(body.len() + 3);
    let enc = target.encoding;
    if enc == UTF_16LE || enc == UTF_16BE {
        let le = enc == UTF_16LE;
        if target.bom {
            out.extend(if le { [0xFF, 0xFE] } else { [0xFE, 0xFF] });
        }
        for u in body.encode_utf16() {
            out.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
    } else {
        if target.bom {
            out.extend([0xEF, 0xBB, 0xBF]); // for_bom only yields UTF-8/UTF-16, so this is the UTF-8 BOM
        }
        let (bytes, _, unmappable) = enc.encode(&body);
        if unmappable {
            crate::trace!("encode: text not representable in {}", enc.name());
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the text has characters {} cannot represent; enable Convert Encoding to save as UTF-8",
                    enc.name()
                ),
            ));
        }
        out.extend(bytes.iter());
    }
    Ok((out, target))
}
