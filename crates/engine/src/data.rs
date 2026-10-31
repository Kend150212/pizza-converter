//! Data conversions: CSV <-> JSON, JSON <-> YAML, XML -> JSON, XLSX -> CSV.
//! Pure Rust, MIT/Apache-2.0 only.

use crate::{ConvertError, Result};
use std::path::Path;

// ---------------------------------------------------------------------------
// CSV <-> JSON
// ---------------------------------------------------------------------------

/// CSV -> JSON array of objects (first row = headers).
pub fn csv_to_json(input: &Path, output: &Path) -> Result<()> {
    let v = csv_to_value(input)?;
    std::fs::write(output, serde_json::to_string_pretty(&v).unwrap())?;
    Ok(())
}

/// CSV -> JSON value (array of objects, first row = headers).
pub fn csv_to_value(input: &Path) -> Result<serde_json::Value> {
    let mut rdr =
        csv::Reader::from_path(input).map_err(|e| ConvertError::Decode(e.to_string()))?;
    let headers = rdr
        .headers()
        .map_err(|e| ConvertError::Decode(e.to_string()))?
        .clone();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| ConvertError::Decode(e.to_string()))?;
        let obj: serde_json::Map<String, serde_json::Value> = headers
            .iter()
            .zip(rec.iter())
            .map(|(h, v)| (h.to_string(), serde_json::Value::String(v.to_string())))
            .collect();
        rows.push(serde_json::Value::Object(obj));
    }
    Ok(serde_json::Value::Array(rows))
}

/// JSON array of objects -> CSV (header = union of keys, first-seen order).
pub fn json_to_csv(input: &Path, output: &Path) -> Result<()> {
    let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(input)?)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    value_to_csv(&data, output)
}

/// Any JSON value -> CSV via the table flattening (`value_to_table`).
/// Non-tabular values become a single "value" column.
pub fn value_to_csv(data: &serde_json::Value, output: &Path) -> Result<()> {
    let (headers, rows) = value_to_table(data);
    let mut wtr =
        csv::Writer::from_path(output).map_err(|e| ConvertError::Encode(e.to_string()))?;
    wtr.write_record(&headers)
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    for row in &rows {
        wtr.write_record(row)
            .map_err(|e| ConvertError::Encode(e.to_string()))?;
    }
    wtr.flush()
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    Ok(())
}


// ---------------------------------------------------------------------------
// JSON <-> YAML
// ---------------------------------------------------------------------------

pub fn json_to_yaml(input: &Path, output: &Path) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(input)?)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    let y = serde_yaml::to_string(&v).map_err(|e| ConvertError::Encode(e.to_string()))?;
    std::fs::write(output, y)?;
    Ok(())
}

pub fn yaml_to_json(input: &Path, output: &Path) -> Result<()> {
    let y: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(input)?)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    let v = serde_json::to_value(y).map_err(|e| ConvertError::Encode(e.to_string()))?;
    std::fs::write(output, serde_json::to_string_pretty(&v).unwrap())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// XML -> JSON
// ---------------------------------------------------------------------------

/// XML -> JSON. Mapping: elements become objects keyed by tag, repeated
/// siblings become arrays, attributes become "@name" keys, pure-text elements
/// become plain strings.
pub fn xml_to_json(input: &Path, output: &Path) -> Result<()> {
    let text = std::fs::read_to_string(input)?;
    let v = xml_to_value(&text)?;
    std::fs::write(output, serde_json::to_string_pretty(&v).unwrap())?;
    Ok(())
}

fn xml_to_value(text: &str) -> Result<serde_json::Value> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<(String, serde_json::Map<String, serde_json::Value>)> = Vec::new();
    let mut root: Option<(String, serde_json::Value)> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let mut map = serde_json::Map::new();
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| ConvertError::Decode(e.to_string()))?;
                    let k = format!(
                        "@{}",
                        String::from_utf8_lossy(attr.key.as_ref())
                    );
                    let v = attr
                        .unescape_value()
                        .map_err(|e| ConvertError::Decode(e.to_string()))?
                        .into_owned();
                    map.insert(k, serde_json::Value::String(v));
                }
                stack.push((name, map));
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                push_value(&mut stack, &mut root, name, serde_json::Value::Null)?;
            }
            Ok(Event::Text(e)) => {
                let t = e
                    .unescape()
                    .map_err(|e| ConvertError::Decode(e.to_string()))?
                    .into_owned();
                if !t.trim().is_empty() {
                    if let Some((_, map)) = stack.last_mut() {
                        map.insert("$text".into(), serde_json::Value::String(t));
                    }
                }
            }
            Ok(Event::End(_)) => {
                let (name, map) = stack
                    .pop()
                    .ok_or_else(|| ConvertError::Decode("unbalanced XML".into()))?;
                let val = if map.len() == 1 && map.contains_key("$text") {
                    map["$text"].clone()
                } else {
                    serde_json::Value::Object(map)
                };
                push_value(&mut stack, &mut root, name, val)?;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ConvertError::Decode(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    root
        .map(|(n, v)| serde_json::json!({ n: v }))
        .ok_or_else(|| ConvertError::Decode("empty XML".into()))
}

fn push_value(
    stack: &mut Vec<(String, serde_json::Map<String, serde_json::Value>)>,
    root: &mut Option<(String, serde_json::Value)>,
    name: String,
    val: serde_json::Value,
) -> Result<()> {
    match stack.last_mut() {
        Some((_, parent)) => {
            match parent.remove(&name) {
                None => {
                    parent.insert(name, val);
                }
                Some(serde_json::Value::Array(mut a)) => {
                    a.push(val);
                    parent.insert(name, serde_json::Value::Array(a));
                }
                Some(prev) => {
                    parent.insert(name, serde_json::Value::Array(vec![prev, val]));
                }
            }
            Ok(())
        }
        None => {
            *root = Some((name, val));
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// XLSX -> CSV (first sheet by default)
// ---------------------------------------------------------------------------

/// Raw string rows from an XLSX sheet (positional — column order preserved).
fn xlsx_rows(input: &Path, sheet: Option<&str>) -> Result<Vec<Vec<String>>> {
    use calamine::{open_workbook_auto, Reader};

    let mut wb =
        open_workbook_auto(input).map_err(|e| ConvertError::Decode(e.to_string()))?;
    let name = match sheet {
        Some(s) => s.to_string(),
        None => wb
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| ConvertError::Decode("workbook has no sheets".into()))?,
    };
    let range = wb
        .worksheet_range(&name)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    Ok(range
        .rows()
        .map(|r| r.iter().map(|c| c.to_string()).collect())
        .collect())
}

pub fn xlsx_to_csv(input: &Path, output: &Path, sheet: Option<&str>) -> Result<()> {
    let rows = xlsx_rows(input, sheet)?;
    let mut wtr =
        csv::Writer::from_path(output).map_err(|e| ConvertError::Encode(e.to_string()))?;
    for r in &rows {
        wtr.write_record(r)
            .map_err(|e| ConvertError::Encode(e.to_string()))?;
    }
    wtr.flush()
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    Ok(())
}

/// XLSX (first sheet, or `sheet`) -> JSON value (array of objects, first row =
/// headers). Empty cells become empty strings. Fully-empty rows are skipped.
pub fn xlsx_to_value(input: &Path, sheet: Option<&str>) -> Result<serde_json::Value> {
    let mut rows = xlsx_rows(input, sheet)?.into_iter();
    let headers: Vec<String> = match rows.next() {
        Some(h) => h,
        None => return Ok(serde_json::Value::Array(vec![])),
    };
    let mut out = Vec::new();
    for row in rows {
        if row.iter().all(|c| c.is_empty()) {
            continue;
        }
        let obj: serde_json::Map<String, serde_json::Value> = headers
            .iter()
            .zip(row.iter())
            .map(|(h, c)| (h.clone(), serde_json::Value::String(c.clone())))
            .collect();
        out.push(serde_json::Value::Object(obj));
    }
    Ok(serde_json::Value::Array(out))
}


// ---------------------------------------------------------------------------
// Hub: every data format converts through serde_json::Value.
// This gives all 5x5 pairs (csv/json/xml/yaml/xlsx) from a small set of
// primitives. Same-extension pairs normalize through the hub (parse +
// re-serialize), which also validates the file.
// ---------------------------------------------------------------------------

/// Read any supported data file into a JSON value. `ext` is lowercased,
/// without the dot.
pub fn read_value(input: &Path, ext: &str) -> Result<serde_json::Value> {
    match ext {
        "csv" => csv_to_value(input),
        "json" => serde_json::from_str(&std::fs::read_to_string(input)?)
            .map_err(|e| ConvertError::Decode(e.to_string())),
        "yaml" | "yml" => {
            let y: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(input)?)
                .map_err(|e| ConvertError::Decode(e.to_string()))?;
            serde_json::to_value(y).map_err(|e| ConvertError::Decode(e.to_string()))
        }
        "xml" => xml_to_value(&std::fs::read_to_string(input)?),
        "xlsx" => xlsx_to_value(input, None),
        _ => Err(ConvertError::UnsupportedInput(ext.into())),
    }
}

/// Write a JSON value to any supported data file. `ext` is lowercased,
/// without the dot.
pub fn write_value(v: &serde_json::Value, output: &Path, ext: &str) -> Result<()> {
    match ext {
        "csv" => value_to_csv(v, output),
        "json" => {
            std::fs::write(output, serde_json::to_string_pretty(v).unwrap())?;
            Ok(())
        }
        "yaml" | "yml" => {
            let y = serde_yaml::to_string(v).map_err(|e| ConvertError::Encode(e.to_string()))?;
            std::fs::write(output, y)?;
            Ok(())
        }
        "xml" => value_to_xml(v, output),
        "xlsx" => value_to_xlsx(v, output),
        _ => Err(ConvertError::UnsupportedOutput(ext.into())),
    }
}

/// Convert between any two data formats (csv/json/xml/yaml/xlsx), both
/// extensions lowercased without the dot.
pub fn convert_data(input: &Path, output: &Path, in_ext: &str, out_ext: &str) -> Result<()> {
    let v = read_value(input, in_ext)?;
    write_value(&v, output, out_ext)
}

/// Normalize a JSON object key for table headers: "@id" -> "id",
/// "$text" -> "text" (XML mapping artifacts).
fn header_name(k: &str) -> String {
    k.strip_prefix('@')
        .or_else(|| k.strip_prefix('$'))
        .unwrap_or(k)
        .to_string()
}

fn value_to_cell(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Flatten a JSON value to (headers, rows) for table formats (CSV/XLSX).
/// - Arrays of objects -> one row per object (union of keys, first-seen).
/// - Single-key objects drill down: {items: [...]} -> the array.
/// - Anything else -> a single "value" column.
fn value_to_table(v: &serde_json::Value) -> (Vec<String>, Vec<Vec<String>>) {
    use serde_json::Value;
    let mut cur = v;
    loop {
        match cur {
            Value::Object(m) if m.len() == 1 => {
                cur = m.values().next().unwrap();
            }
            _ => break,
        }
    }
    match cur {
        Value::Array(arr) => {
            let mut headers: Vec<String> = Vec::new();
            for el in arr {
                if let Value::Object(o) = el {
                    for k in o.keys() {
                        let h = header_name(k);
                        if !headers.contains(&h) {
                            headers.push(h);
                        }
                    }
                }
            }
            if headers.is_empty() {
                headers.push("value".to_string());
            }
            let rows = arr
                .iter()
                .map(|el| match el {
                    Value::Object(o) => headers
                        .iter()
                        .map(|h| {
                            o.iter()
                                .find(|(k, _)| header_name(k) == *h)
                                .map(|(_, val)| value_to_cell(val))
                                .unwrap_or_default()
                        })
                        .collect(),
                    other => vec![value_to_cell(other)],
                })
                .collect();
            (headers, rows)
        }
        Value::Object(m) => {
            let headers: Vec<String> = m.keys().map(|k| header_name(k)).collect();
            let row: Vec<String> = m.values().map(value_to_cell).collect();
            (headers, vec![row])
        }
        scalar => (vec!["value".to_string()], vec![vec![value_to_cell(scalar)]]),
    }
}

/// JSON value -> XLSX (first sheet gets the flattened table).
pub fn value_to_xlsx(v: &serde_json::Value, output: &Path) -> Result<()> {
    use rust_xlsxwriter::Workbook;
    let (headers, rows) = value_to_table(v);
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (c, h) in headers.iter().enumerate() {
        ws.write_string(0, c as u16, h)
            .map_err(|e| ConvertError::Encode(e.to_string()))?;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, cell)
                .map_err(|e| ConvertError::Encode(e.to_string()))?;
        }
    }
    wb.save(output)
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    Ok(())
}

fn xml_escape(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Make a string safe as an XML tag/attribute name.
fn xml_tag(t: &str) -> String {
    let mut s: String = t
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s = format!("_{s}");
    }
    s
}

/// Recursive XML writer. Object "@key" entries become attributes, "$text"
/// becomes text content, arrays repeat the element tag.
fn write_xml_value(s: &mut String, tag: &str, v: &serde_json::Value) {
    use serde_json::Value;
    let tag = xml_tag(tag);
    match v {
        Value::Object(m) => {
            let mut attrs = String::new();
            let mut text: Option<&str> = None;
            let mut children: Vec<(&String, &Value)> = Vec::new();
            for (k, val) in m {
                if let Some(a) = k.strip_prefix('@') {
                    attrs.push_str(&format!(
                        " {}=\"{}\"",
                        xml_tag(a),
                        xml_escape(&value_to_cell(val))
                    ));
                } else if k == "$text" {
                    text = val.as_str();
                } else {
                    children.push((k, val));
                }
            }
            s.push_str(&format!("<{tag}{attrs}"));
            let has_body = !children.is_empty() || text.map(|t| !t.is_empty()).unwrap_or(false);
            if !has_body {
                s.push_str("/>");
                return;
            }
            s.push('>');
            if let Some(t) = text {
                s.push_str(&xml_escape(t));
            }
            for (k, val) in children {
                match val {
                    Value::Array(a) => {
                        for el in a {
                            write_xml_value(s, k, el);
                        }
                    }
                    _ => write_xml_value(s, k, val),
                }
            }
            s.push_str(&format!("</{tag}>"));
        }
        Value::Array(a) => {
            for el in a {
                write_xml_value(s, &tag, el);
            }
        }
        _ => {
            s.push_str(&format!("<{tag}>{}</{tag}>", xml_escape(&value_to_cell(v))));
        }
    }
}

/// JSON value -> XML document.
pub fn value_to_xml(v: &serde_json::Value, output: &Path) -> Result<()> {
    use serde_json::Value;
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    match v {
        Value::Array(a) => {
            s.push_str("<root>");
            for el in a {
                write_xml_value(&mut s, "item", el);
            }
            s.push_str("</root>");
        }
        Value::Object(m) if m.len() == 1 => {
            let (k, val) = m.iter().next().unwrap();
            match val {
                // A root-level array must be wrapped in ONE element;
                // emitting one <key> per item would create multiple roots.
                Value::Array(a) => {
                    let tag = xml_tag(k);
                    s.push_str(&format!("<{tag}>"));
                    for el in a {
                        write_xml_value(&mut s, "item", el);
                    }
                    s.push_str(&format!("</{tag}>"));
                }
                _ => write_xml_value(&mut s, k, val),
            }
        }
        _ => {
            s.push_str("<root>");
            write_xml_value(&mut s, "value", v);
            s.push_str("</root>");
        }
    }
    std::fs::write(output, s)?;
    Ok(())
}

/// JSON value -> XML file.
pub fn json_to_xml(input: &Path, output: &Path) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(input)?)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    value_to_xml(&v, output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn sample_table_csv(dir: &std::path::Path) -> std::path::PathBuf {
        let p = dir.join("t.csv");
        std::fs::write(&p, "id,name\n1,An\n2,Binh\n").unwrap();
        p
    }

    #[test]
    fn hub_csv_xlsx_roundtrip() {
        let dir = tmp();
        let csv_p = sample_table_csv(dir.path());
        let xlsx_p = dir.path().join("t.xlsx");
        convert_data(&csv_p, &xlsx_p, "csv", "xlsx").unwrap();
        assert!(xlsx_p.is_file());
        let back = dir.path().join("b.csv");
        convert_data(&xlsx_p, &back, "xlsx", "csv").unwrap();
        let text = std::fs::read_to_string(&back).unwrap();
        assert!(text.contains("An") && text.contains("Binh"));
    }

    #[test]
    fn hub_json_xml_yaml() {
        let dir = tmp();
        let json_p = dir.path().join("t.json");
        std::fs::write(&json_p, r#"[{"id":"1","name":"An"}]"#).unwrap();
        let xml_p = dir.path().join("t.xml");
        convert_data(&json_p, &xml_p, "json", "xml").unwrap();
        let xml = std::fs::read_to_string(&xml_p).unwrap();
        assert!(xml.contains("<name>An</name>"));
        let yaml_p = dir.path().join("t.yaml");
        convert_data(&xml_p, &yaml_p, "xml", "yaml").unwrap();
        let y = std::fs::read_to_string(&yaml_p).unwrap();
        assert!(y.contains("An"));
        // self-pairs normalize
        let back = dir.path().join("b.json");
        convert_data(&json_p, &back, "json", "json").unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&back).unwrap()).unwrap();
        assert_eq!(v[0]["name"], "An");
    }

    #[test]
    fn hub_xlsx_self_and_value() {
        let dir = tmp();
        let csv_p = sample_table_csv(dir.path());
        let xlsx_p = dir.path().join("t.xlsx");
        convert_data(&csv_p, &xlsx_p, "csv", "xlsx").unwrap();
        let v = xlsx_to_value(&xlsx_p, None).unwrap();
        assert_eq!(v[0]["name"], "An");
        assert_eq!(v[1]["id"], "2");
        let copy = dir.path().join("c.xlsx");
        convert_data(&xlsx_p, &copy, "xlsx", "xlsx").unwrap();
        assert!(copy.is_file());
    }

    #[test]
    fn hub_yaml_csv_and_xml_attr() {
        let dir = tmp();
        // table nested under one key, like the QA sample
        let yaml_p = dir.path().join("t.yaml");
        std::fs::write(&yaml_p, "items:\n  - id: 1\n    name: An\n").unwrap();
        let csv_p = dir.path().join("t.csv");
        convert_data(&yaml_p, &csv_p, "yaml", "csv").unwrap();
        let text = std::fs::read_to_string(&csv_p).unwrap();
        assert!(text.lines().next().unwrap().contains("id"));
        assert!(text.contains("An"));
    }

    #[test]
    fn csv_json_roundtrip() {
        let dir = tmp();
        let csv_p = dir.path().join("a.csv");
        let json_p = dir.path().join("a.json");
        let back_p = dir.path().join("b.csv");
        std::fs::write(&csv_p, "name,age\nAn,30\nBinh,25\n").unwrap();
        csv_to_json(&csv_p, &json_p).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_p).unwrap()).unwrap();
        assert_eq!(v[0]["name"], "An");
        assert_eq!(v[1]["age"], "25");
        json_to_csv(&json_p, &back_p).unwrap();
        let back = std::fs::read_to_string(&back_p).unwrap();
        // Header order follows serde_json::Map (alphabetical); check content, not order.
        let mut lines = back.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert!(header.contains(&"name") && header.contains(&"age"));
        let body: Vec<&str> = lines.collect();
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn yaml_json_roundtrip() {
        let dir = tmp();
        let json_p = dir.path().join("a.json");
        let yaml_p = dir.path().join("a.yaml");
        let back_p = dir.path().join("b.json");
        std::fs::write(&json_p, r#"{"a":1,"b":[1,2]}"#).unwrap();
        json_to_yaml(&json_p, &yaml_p).unwrap();
        let y = std::fs::read_to_string(&yaml_p).unwrap();
        assert!(y.contains("a: 1"));
        yaml_to_json(&yaml_p, &back_p).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&back_p).unwrap()).unwrap();
        assert_eq!(v["b"], serde_json::json!([1, 2]));
    }

    #[test]
    fn xml_to_json_shapes() {
        let dir = tmp();
        let xml_p = dir.path().join("a.xml");
        let json_p = dir.path().join("a.json");
        std::fs::write(
            &xml_p,
            r#"<root><item id="1">one</item><item id="2">two</item><single>x</single></root>"#,
        )
        .unwrap();
        xml_to_json(&xml_p, &json_p).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_p).unwrap()).unwrap();
        assert_eq!(v["root"]["item"][0]["@id"], "1");
        assert_eq!(v["root"]["item"][1]["$text"], "two");
        assert_eq!(v["root"]["single"], "x");
    }

    #[test]
    fn xlsx_to_csv_first_sheet() {
        let dir = tmp();
        let xlsx_p = dir.path().join("a.xlsx");
        // Fixture via openpyxl (dev machine); calamine reads it back.
        let code = format!(
            "import openpyxl; wb=openpyxl.Workbook(); ws=wb.active; \
             ws.append(['name','age']); ws.append(['An',30]); \
             wb.save(r'{}')",
            xlsx_p.to_string_lossy()
        );
        let st = std::process::Command::new("python3")
            .args(["-c", &code])
            .status()
            .unwrap();
        assert!(st.success());
        let csv_p = dir.path().join("a.csv");
        xlsx_to_csv(&xlsx_p, &csv_p, None).unwrap();
        let out = std::fs::read_to_string(&csv_p).unwrap();
        assert!(out.contains("name,age"));
        assert!(out.contains("An,30"));
    }
}
