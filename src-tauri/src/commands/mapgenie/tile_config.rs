//! Parsing of the "tile config" that MapGenie hides inside each map's web page.
//!
//! Cybersecurity note: we are basically *web scraping* here — reading data out of
//! someone else's HTML page. That data is untrusted input, and worse, different
//! games encode it in different shapes. So we write a careful custom deserializer
//! that accepts every shape we have seen and normalises it. Being strict about
//! "what input do I actually accept?" is exactly the defensive-parsing mindset
//! from class: never assume the input looks the way you hope.

use serde::Deserialize;
use std::collections::HashMap;

/// A simple inclusive range (smallest and biggest allowed value).
#[derive(Debug, Deserialize)]
pub struct MinMax {
    pub min: u32,
    pub max: u32,
}

/// Which tile columns (x) and rows (y) actually exist at one zoom level.
#[derive(Debug, Deserialize)]
pub struct TileSetBounds {
    pub x: MinMax,
    pub y: MinMax,
}

/// Deserializes tile bounds, which MapGenie encodes inconsistently across games:
///   - an object keyed by zoom-level string (e.g. {"3": {x,y}, ...}) — most games
///   - an array indexed by zoom level (e.g. [{x,y}, {x,y}, ...]) — e.g. Far Cry New Dawn
///   - null — maps with no per-zoom bounds
/// All forms are normalized to a HashMap keyed by zoom-level string. Array indices become
/// the zoom keys. The visitor MUST fully drain a sequence, or serde_json errors with
/// "invalid length N, expected fewer elements in array".
fn deserialize_tile_bounds<'de, D>(d: D) -> Result<HashMap<String, TileSetBounds>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{MapAccess, SeqAccess, Visitor};
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = HashMap<String, TileSetBounds>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a map, null, or sequence")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
            let mut map = HashMap::new();
            while let Some((k, v)) = a.next_entry::<String, TileSetBounds>()? {
                map.insert(k, v);
            }
            Ok(map)
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(HashMap::new()) }
        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(HashMap::new()) }
        fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
            // Array form: index = zoom level. Drain fully and key by index.
            let mut map = HashMap::new();
            let mut idx = 0usize;
            while let Some(v) = a.next_element::<Option<TileSetBounds>>()? {
                if let Some(b) = v {
                    map.insert(idx.to_string(), b);
                }
                idx += 1;
            }
            Ok(map)
        }
    }
    d.deserialize_any(V)
}

/// Describes one set of tiles: the URL pattern, the file type, the zoom range,
/// and the per-zoom bounds we computed above.
#[derive(Debug, Deserialize)]
pub struct TileSet {
    pub pattern: String,
    pub extension: String,
    pub min_zoom: u32,
    pub max_zoom: u32,
    #[serde(deserialize_with = "deserialize_tile_bounds")]
    pub bounds: HashMap<String, TileSetBounds>,
}

/// The `mapConfig` object — a map usually has exactly one tile set.
#[derive(Debug, Deserialize)]
pub struct MapConfig {
    pub tile_sets: Vec<TileSet>,
}

/// The top-level `window.mapData` object we scrape from the page.
#[derive(Debug, Deserialize)]
pub struct MapDataHtml {
    #[serde(rename = "mapConfig")]
    pub map_config: MapConfig,
}
