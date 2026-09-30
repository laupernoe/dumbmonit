//! Le sous-ensemble de BSON dont une sonde a besoin.
//!
//! Écrire : un document de commande, fait de chaînes, d'entiers, de booléens,
//! de données binaires (la charge SASL) et de sous-documents. Lire : tout ce
//! que `serverStatus` et `replSetGetStatus` peuvent rendre, y compris les types
//! qu'aucune mesure n'emploie (identifiants d'objet, décimaux 128 bits), pour
//! pouvoir les sauter sans perdre le fil du document.
//!
//! Spécification : <https://bsonspec.org/spec.html>.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Bson {
    Double(f64),
    String(String),
    Document(Document),
    Array(Vec<Bson>),
    Binary(Vec<u8>),
    ObjectId([u8; 12]),
    Bool(bool),
    /// Millisecondes depuis l'époque Unix.
    DateTime(i64),
    Null,
    Int32(i32),
    Timestamp {
        time: u32,
        increment: u32,
    },
    Int64(i64),
    /// Types lus mais sans usage ici (décimal 128, expression rationnelle, code…).
    Other,
}

impl Bson {
    /// Valeur numérique, quel que soit le type entier ou flottant employé :
    /// MongoDB passe d'`int32` à `int64` au gré de la taille d'un compteur.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Double(v) => Some(*v),
            Self::Int32(v) => Some(f64::from(*v)),
            Self::Int64(v) => Some(*v as f64),
            Self::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_document(&self) -> Option<&Document> {
        match self {
            Self::Document(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document(pub Vec<(String, Bson)>);

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, key: &str, value: Bson) -> Self {
        self.0.push((key.to_string(), value));
        self
    }

    pub fn get(&self, key: &str) -> Option<&Bson> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Valeur au bout d'un chemin pointé : `wiredTiger.cache.bytes currently in the cache`.
    /// Le séparateur est `/` pour autoriser les points et espaces des clés WiredTiger.
    pub fn path(&self, path: &str) -> Option<&Bson> {
        let mut parts = path.split('/');
        let mut current = self.get(parts.next()?)?;
        for part in parts {
            current = current.as_document()?.get(part)?;
        }
        Some(current)
    }

    pub fn number(&self, path: &str) -> Option<f64> {
        self.path(path)?.as_f64().filter(|v| v.is_finite())
    }

    pub fn str(&self, path: &str) -> Option<&str> {
        self.path(path)?.as_str()
    }

    pub fn doc(&self, path: &str) -> Option<&Document> {
        self.path(path)?.as_document()
    }

    pub fn array(&self, path: &str) -> Option<&[Bson]> {
        match self.path(path)? {
            Bson::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0u8; 4];
        for (key, value) in &self.0 {
            encode_element(&mut out, key, value);
        }
        out.push(0);
        let length = out.len() as i32;
        out[..4].copy_from_slice(&length.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader { bytes, pos: 0 };
        let document = reader.document(0)?;
        Ok(document)
    }
}

fn encode_element(out: &mut Vec<u8>, key: &str, value: &Bson) {
    let (kind, body): (u8, Vec<u8>) = match value {
        Bson::Double(v) => (0x01, v.to_le_bytes().to_vec()),
        Bson::String(v) => (0x02, string_bytes(v)),
        Bson::Document(v) => (0x03, v.encode()),
        Bson::Array(items) => {
            let doc = Document(
                items.iter().enumerate().map(|(i, item)| (i.to_string(), item.clone())).collect(),
            );
            (0x04, doc.encode())
        }
        Bson::Binary(v) => {
            let mut body = (v.len() as i32).to_le_bytes().to_vec();
            body.push(0x00);
            body.extend_from_slice(v);
            (0x05, body)
        }
        Bson::ObjectId(v) => (0x07, v.to_vec()),
        Bson::Bool(v) => (0x08, vec![u8::from(*v)]),
        Bson::DateTime(v) => (0x09, v.to_le_bytes().to_vec()),
        Bson::Null | Bson::Other => (0x0A, Vec::new()),
        Bson::Int32(v) => (0x10, v.to_le_bytes().to_vec()),
        Bson::Timestamp { time, increment } => {
            let mut body = increment.to_le_bytes().to_vec();
            body.extend_from_slice(&time.to_le_bytes());
            (0x11, body)
        }
        Bson::Int64(v) => (0x12, v.to_le_bytes().to_vec()),
    };
    out.push(kind);
    out.extend_from_slice(key.as_bytes());
    out.push(0);
    out.extend_from_slice(&body);
}

fn string_bytes(value: &str) -> Vec<u8> {
    let mut body = ((value.len() + 1) as i32).to_le_bytes().to_vec();
    body.extend_from_slice(value.as_bytes());
    body.push(0);
    body
}

#[derive(Debug, PartialEq)]
pub struct DecodeError(pub String);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed BSON: {}", self.0)
    }
}

/// Profondeur maximale d'imbrication : `serverStatus` descend à six niveaux.
const MAX_DEPTH: usize = 64;

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], DecodeError> {
        let end = self.pos.checked_add(n).filter(|end| *end <= self.bytes.len());
        let end = end.ok_or_else(|| DecodeError(format!("truncated at byte {}", self.pos)))?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap_or_default()))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap_or_default()))
    }

    fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap_or_default()))
    }

    fn length(&mut self) -> Result<usize, DecodeError> {
        usize::try_from(self.i32()?).map_err(|_| DecodeError("negative length".to_string()))
    }

    fn cstring(&mut self) -> Result<String, DecodeError> {
        let rest = &self.bytes[self.pos..];
        let end = rest
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| DecodeError("unterminated key".to_string()))?;
        let text = String::from_utf8_lossy(&rest[..end]).into_owned();
        self.pos += end + 1;
        Ok(text)
    }

    fn string(&mut self) -> Result<String, DecodeError> {
        let length = self.length()?;
        if length == 0 {
            return Err(DecodeError("empty string length".to_string()));
        }
        let bytes = self.take(length)?;
        Ok(String::from_utf8_lossy(&bytes[..length - 1]).into_owned())
    }

    fn document(&mut self, depth: usize) -> Result<Document, DecodeError> {
        if depth > MAX_DEPTH {
            return Err(DecodeError("nested too deep".to_string()));
        }
        let start = self.pos;
        let length = self.length()?;
        if length < 5 || start + length > self.bytes.len() {
            return Err(DecodeError(format!("document length {length} out of bounds")));
        }
        let end = start + length - 1;
        let mut elements = Vec::new();
        while self.pos < end {
            let kind = self.u8()?;
            let key = self.cstring()?;
            let value = self.value(kind, depth)?;
            elements.push((key, value));
        }
        if self.u8()? != 0 {
            return Err(DecodeError("document not terminated".to_string()));
        }
        Ok(Document(elements))
    }

    fn value(&mut self, kind: u8, depth: usize) -> Result<Bson, DecodeError> {
        Ok(match kind {
            0x01 => Bson::Double(f64::from_le_bytes(self.take(8)?.try_into().unwrap_or_default())),
            0x02 => Bson::String(self.string()?),
            0x03 => Bson::Document(self.document(depth + 1)?),
            0x04 => Bson::Array(self.document(depth + 1)?.0.into_iter().map(|(_, v)| v).collect()),
            0x05 => {
                let length = self.length()?;
                let _subtype = self.u8()?;
                Bson::Binary(self.take(length)?.to_vec())
            }
            0x06 | 0x0A | 0x7F | 0xFF => Bson::Null,
            0x07 => {
                let mut id = [0u8; 12];
                id.copy_from_slice(self.take(12)?);
                Bson::ObjectId(id)
            }
            0x08 => Bson::Bool(self.u8()? != 0),
            0x09 => Bson::DateTime(self.i64()?),
            0x0B => {
                self.cstring()?;
                self.cstring()?;
                Bson::Other
            }
            0x0C => {
                self.string()?;
                self.take(12)?;
                Bson::Other
            }
            0x0D | 0x0E => {
                self.string()?;
                Bson::Other
            }
            0x0F => {
                let length = self.length()?;
                self.take(length.saturating_sub(4))?;
                Bson::Other
            }
            0x10 => Bson::Int32(self.i32()?),
            0x11 => {
                let increment = self.u32()?;
                let time = self.u32()?;
                Bson::Timestamp { time, increment }
            }
            0x12 => Bson::Int64(self.i64()?),
            0x13 => {
                self.take(16)?;
                Bson::Other
            }
            other => return Err(DecodeError(format!("unknown element type 0x{other:02x}"))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let doc = Document::new()
            .with("saslStart", Bson::Int32(1))
            .with("mechanism", Bson::String("SCRAM-SHA-256".into()))
            .with("payload", Bson::Binary(b"n,,n=u,r=abc".to_vec()))
            .with("options", Bson::Document(Document::new().with("skip", Bson::Bool(true))))
            .with("big", Bson::Int64(1 << 40))
            .with("ratio", Bson::Double(0.5))
            .with("list", Bson::Array(vec![Bson::Int32(1), Bson::String("x".into())]))
            .with("when", Bson::DateTime(1_790_750_921_000))
            .with("ts", Bson::Timestamp { time: 7, increment: 1 });
        let bytes = doc.encode();
        assert_eq!(Document::decode(&bytes).unwrap(), doc);
    }

    /// `{"hello": "world"}` tel que le donne la spécification.
    #[test]
    fn l_exemple_de_la_specification() {
        let bytes = b"\x16\x00\x00\x00\x02hello\x00\x06\x00\x00\x00world\x00\x00";
        let doc = Document::decode(bytes).unwrap();
        assert_eq!(doc.str("hello"), Some("world"));
        assert_eq!(Document::new().with("hello", Bson::String("world".into())).encode(), bytes);
    }

    #[test]
    fn un_document_tronque_est_refuse_sans_paniquer() {
        let bytes = Document::new().with("a", Bson::String("bcdef".into())).encode();
        for cut in 0..bytes.len() {
            assert!(Document::decode(&bytes[..cut]).is_err(), "coupé à {cut}");
        }
    }

    #[test]
    fn les_chemins_traversent_les_cles_avec_espaces() {
        let doc = Document::new().with(
            "wiredTiger",
            Bson::Document(Document::new().with(
                "cache",
                Bson::Document(
                    Document::new().with("bytes currently in the cache", Bson::Int64(42)),
                ),
            )),
        );
        assert_eq!(doc.number("wiredTiger/cache/bytes currently in the cache"), Some(42.0));
        assert_eq!(doc.number("wiredTiger/absent"), None);
    }
}
