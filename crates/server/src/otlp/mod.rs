//! Réception OpenTelemetry (OTLP/HTTP) : les messages du protocole, lus en
//! protobuf ou en JSON, puis convertis en séries (`convert`).
//!
//! Seul le sous-ensemble du protocole utile aux métriques est décrit ici
//! (`opentelemetry/proto/metrics/v1/metrics.proto`, `common/v1`, `resource/v1`) :
//! le crate `opentelemetry-proto` embarquerait le SDK et `tonic` pour quelques
//! structures. Les mêmes types portent les deux encodages — `prost` pour le
//! protobuf, `serde` pour le JSON canonique d'OTLP (noms en camelCase, entiers
//! 64 bits écrits en chaînes ou en nombres, énumérations en entier ou en nom). Un
//! champ que le protocole ajoutera plus tard est ignoré, dans les deux encodages.
//!
//! Les traces, les exemplaires et les histogrammes exponentiels ne sont pas
//! stockés : ils sont comptés comme rejetés dans l'accusé de réception.

pub mod convert;

use std::io::Read;

use prost::Message;
use serde::{Deserialize, Deserializer};

/// Taille maximale d'un corps une fois décompressé.
pub const MAX_DECOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;

// --------------------------------------------------------------------------
// Lecture tolérante des nombres JSON
// --------------------------------------------------------------------------

/// Un nombre JSON d'OTLP : les entiers 64 bits sont écrits en chaînes par
/// beaucoup d'exportateurs, en nombres par d'autres.
#[derive(Deserialize)]
#[serde(untagged)]
enum Flex {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Text(String),
}

fn flex_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    match Flex::deserialize(deserializer)? {
        Flex::Unsigned(n) => Ok(n),
        Flex::Signed(n) => u64::try_from(n).map_err(serde::de::Error::custom),
        Flex::Float(f) if f >= 0.0 => Ok(f as u64),
        Flex::Float(_) => Err(serde::de::Error::custom("negative number")),
        Flex::Text(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

fn flex_i64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    match Flex::deserialize(deserializer)? {
        Flex::Unsigned(n) => i64::try_from(n).map_err(serde::de::Error::custom),
        Flex::Signed(n) => Ok(n),
        Flex::Float(f) => Ok(f as i64),
        Flex::Text(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

fn flex_u64_vec<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u64>, D::Error> {
    #[derive(Deserialize)]
    struct One(#[serde(deserialize_with = "flex_u64")] u64);
    let items = Vec::<One>::deserialize(deserializer)?;
    Ok(items.into_iter().map(|One(n)| n).collect())
}

/// Des octets (en base64 dans le JSON) dont on ne garde rien : une étiquette
/// n'est pas un blob.
fn ignore_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    String::deserialize(deserializer).map(|_| Vec::new())
}

/// `AggregationTemporality` : un entier, ou le nom de la valeur.
fn flex_temporality<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i32, D::Error> {
    match Flex::deserialize(deserializer)? {
        Flex::Unsigned(n) => Ok(n as i32),
        Flex::Signed(n) => Ok(n as i32),
        Flex::Float(f) => Ok(f as i32),
        Flex::Text(s) if s.contains("CUMULATIVE") => Ok(TEMPORALITY_CUMULATIVE),
        Flex::Text(s) if s.contains("DELTA") => Ok(TEMPORALITY_DELTA),
        Flex::Text(_) => Ok(0),
    }
}

pub const TEMPORALITY_DELTA: i32 = 1;
pub const TEMPORALITY_CUMULATIVE: i32 = 2;

// --------------------------------------------------------------------------
// Messages
// --------------------------------------------------------------------------

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportMetricsServiceRequest {
    #[prost(message, repeated, tag = "1")]
    pub resource_metrics: Vec<ResourceMetrics>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ResourceMetrics {
    #[prost(message, optional, tag = "1")]
    pub resource: Option<Resource>,
    #[prost(message, repeated, tag = "2")]
    pub scope_metrics: Vec<ScopeMetrics>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Resource {
    #[prost(message, repeated, tag = "1")]
    pub attributes: Vec<KeyValue>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScopeMetrics {
    #[prost(message, repeated, tag = "2")]
    pub metrics: Vec<Metric>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Metric {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub unit: String,
    #[prost(oneof = "metric::Data", tags = "5, 7, 9, 10, 11")]
    #[serde(flatten)]
    pub data: Option<metric::Data>,
}

pub mod metric {
    use prost::Oneof;
    use serde::Deserialize;

    use super::{ExponentialHistogram, Gauge, Histogram, Sum, Summary};

    #[derive(Clone, PartialEq, Oneof, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum Data {
        #[prost(message, tag = "5")]
        Gauge(Gauge),
        #[prost(message, tag = "7")]
        Sum(Sum),
        #[prost(message, tag = "9")]
        Histogram(Histogram),
        #[prost(message, tag = "10")]
        ExponentialHistogram(ExponentialHistogram),
        #[prost(message, tag = "11")]
        Summary(Summary),
    }
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Gauge {
    #[prost(message, repeated, tag = "1")]
    pub data_points: Vec<NumberDataPoint>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Sum {
    #[prost(message, repeated, tag = "1")]
    pub data_points: Vec<NumberDataPoint>,
    #[prost(int32, tag = "2")]
    #[serde(deserialize_with = "flex_temporality")]
    pub aggregation_temporality: i32,
    #[prost(bool, tag = "3")]
    pub is_monotonic: bool,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Histogram {
    #[prost(message, repeated, tag = "1")]
    pub data_points: Vec<HistogramDataPoint>,
    #[prost(int32, tag = "2")]
    #[serde(deserialize_with = "flex_temporality")]
    pub aggregation_temporality: i32,
}

/// Les points sont seulement comptés : un histogramme exponentiel n'est pas
/// converti (ses seaux n'ont pas d'équivalent en `le` classique).
#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExponentialHistogram {
    #[prost(message, repeated, tag = "1")]
    pub data_points: Vec<Ignored>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default)]
pub struct Ignored {}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Summary {
    #[prost(message, repeated, tag = "1")]
    pub data_points: Vec<SummaryDataPoint>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NumberDataPoint {
    #[prost(message, repeated, tag = "7")]
    pub attributes: Vec<KeyValue>,
    #[prost(fixed64, tag = "3")]
    #[serde(deserialize_with = "flex_u64")]
    pub time_unix_nano: u64,
    #[prost(oneof = "number_data_point::Value", tags = "4, 6")]
    #[serde(flatten)]
    pub value: Option<number_data_point::Value>,
}

pub mod number_data_point {
    use prost::Oneof;
    use serde::Deserialize;

    use super::flex_i64;

    #[derive(Clone, PartialEq, Oneof, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum Value {
        #[prost(double, tag = "4")]
        AsDouble(f64),
        #[prost(sfixed64, tag = "6")]
        AsInt(#[serde(deserialize_with = "flex_i64")] i64),
    }
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistogramDataPoint {
    #[prost(message, repeated, tag = "9")]
    pub attributes: Vec<KeyValue>,
    #[prost(fixed64, tag = "3")]
    #[serde(deserialize_with = "flex_u64")]
    pub time_unix_nano: u64,
    #[prost(fixed64, tag = "4")]
    #[serde(deserialize_with = "flex_u64")]
    pub count: u64,
    #[prost(double, optional, tag = "5")]
    pub sum: Option<f64>,
    #[prost(fixed64, repeated, tag = "6")]
    #[serde(deserialize_with = "flex_u64_vec")]
    pub bucket_counts: Vec<u64>,
    #[prost(double, repeated, tag = "7")]
    pub explicit_bounds: Vec<f64>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SummaryDataPoint {
    #[prost(message, repeated, tag = "7")]
    pub attributes: Vec<KeyValue>,
    #[prost(fixed64, tag = "3")]
    #[serde(deserialize_with = "flex_u64")]
    pub time_unix_nano: u64,
    #[prost(fixed64, tag = "4")]
    #[serde(deserialize_with = "flex_u64")]
    pub count: u64,
    #[prost(double, tag = "5")]
    pub sum: f64,
    #[prost(message, repeated, tag = "6")]
    pub quantile_values: Vec<ValueAtQuantile>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ValueAtQuantile {
    #[prost(double, tag = "1")]
    pub quantile: f64,
    #[prost(double, tag = "2")]
    pub value: f64,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct KeyValue {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(message, optional, tag = "2")]
    pub value: Option<AnyValue>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AnyValue {
    #[prost(oneof = "any_value::Value", tags = "1, 2, 3, 4, 5, 6, 7")]
    #[serde(flatten)]
    pub value: Option<any_value::Value>,
}

pub mod any_value {
    use prost::Oneof;
    use serde::Deserialize;

    use super::{ArrayValue, KeyValueList, flex_i64, ignore_text};

    #[derive(Clone, PartialEq, Oneof, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum Value {
        #[prost(string, tag = "1")]
        StringValue(String),
        #[prost(bool, tag = "2")]
        BoolValue(bool),
        #[prost(int64, tag = "3")]
        IntValue(#[serde(deserialize_with = "flex_i64")] i64),
        #[prost(double, tag = "4")]
        DoubleValue(f64),
        #[prost(message, tag = "5")]
        ArrayValue(ArrayValue),
        #[prost(message, tag = "6")]
        KvlistValue(KeyValueList),
        #[prost(bytes = "vec", tag = "7")]
        BytesValue(#[serde(deserialize_with = "ignore_text")] Vec<u8>),
    }
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArrayValue {
    #[prost(message, repeated, tag = "1")]
    pub values: Vec<AnyValue>,
}

#[derive(Clone, PartialEq, Message, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct KeyValueList {
    #[prost(message, repeated, tag = "1")]
    pub values: Vec<KeyValue>,
}

/// Accusé de réception : `partial_success.rejected_data_points`.
#[derive(Clone, PartialEq, Message)]
pub struct ExportMetricsServiceResponse {
    #[prost(message, optional, tag = "1")]
    pub partial_success: Option<ExportMetricsPartialSuccess>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ExportMetricsPartialSuccess {
    #[prost(int64, tag = "1")]
    pub rejected_data_points: i64,
    #[prost(string, tag = "2")]
    pub error_message: String,
}

impl ExportMetricsServiceResponse {
    /// Accusé protobuf ; vide quand tout a été retenu.
    pub fn protobuf(rejected: usize, why: &str) -> Vec<u8> {
        let partial_success = (rejected > 0).then(|| ExportMetricsPartialSuccess {
            rejected_data_points: rejected as i64,
            error_message: why.to_string(),
        });
        Self { partial_success }.encode_to_vec()
    }

    /// Accusé JSON, même contenu.
    pub fn json(rejected: usize, why: &str) -> serde_json::Value {
        if rejected == 0 {
            return serde_json::json!({});
        }
        serde_json::json!({
            "partialSuccess": { "rejectedDataPoints": rejected, "errorMessage": why }
        })
    }
}

// --------------------------------------------------------------------------
// Décodage du corps
// --------------------------------------------------------------------------

/// Encodage du corps, d'après `Content-Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Protobuf,
    Json,
}

impl Format {
    /// `None` pour un type que l'on ne sait pas lire (415).
    pub fn from_content_type(raw: Option<&str>) -> Option<Self> {
        let essence = raw?.split(';').next()?.trim().to_ascii_lowercase();
        match essence.as_str() {
            "application/x-protobuf" | "application/protobuf" => Some(Self::Protobuf),
            "application/json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Décompresse un corps `gzip` en bornant la taille obtenue : un petit corps ne
/// doit pas pouvoir se déployer en gigaoctets.
pub fn gunzip(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut reader = flate2::read::GzDecoder::new(body).take(MAX_DECOMPRESSED_BYTES + 1);
    reader.read_to_end(&mut out).map_err(|error| format!("Invalid gzip body: {error}"))?;
    if out.len() as u64 > MAX_DECOMPRESSED_BYTES {
        return Err(format!(
            "The decompressed body exceeds {} MiB.",
            MAX_DECOMPRESSED_BYTES / (1024 * 1024)
        ));
    }
    Ok(out)
}

impl ExportMetricsServiceRequest {
    pub fn decode_body(body: &[u8], format: Format) -> Result<Self, String> {
        match format {
            Format::Protobuf => {
                Self::decode(body).map_err(|error| format!("Invalid protobuf: {error}"))
            }
            Format::Json => {
                serde_json::from_slice(body).map_err(|error| format!("Invalid JSON: {error}"))
            }
        }
    }
}
