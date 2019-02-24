use attribute::Attribute;

use csv;
use util;

use xaxis::XAxis;
use Opt;

use serde_json;
use std::convert::From;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::SeekFrom;
use tempfile;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JSONTopic {
    pub name: String,
    pub unit: String,
    pub attrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JSONValue {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JSONHeader {
    pub topics: Vec<JSONTopic>,
    pub values: Vec<JSONValue>,
}

#[derive(Debug)]
struct MidLevelInput {
    pub json_header: Option<JSONHeader>,
    pub body: Vec<(String, Vec<String>)>,
    pub json_header_text: Option<String>,
    pub csv_text: String,
}

#[derive(Debug)]
pub struct Topic {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: String,
    pub attrs: Vec<Attribute>,
    pub data: Vec<f64>,
}

#[derive(Debug)]
pub struct Log {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: String,
    pub attrs: Vec<Attribute>,
    pub data: Vec<(u64, String)>,
    pub lines: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Value {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub value: String,
}

#[derive(Debug)]
pub struct Input {
    pub topics: Vec<Topic>,
    pub logs: Vec<Log>,
    pub values: Vec<Value>,
    pub json_header_text: Option<String>,
    pub csv_text: String,
}

#[derive(Debug)]
enum ParseMode {
    Bag(JSONHeader),
    Csv,
}

impl From<&JSONValue> for Value {
    fn from(value: &JSONValue) -> Self {
        let (folder, base) = util::split_name(&value.name);
        Value {
            name: value.name.clone(),
            name_base: base,
            name_folder: folder,
            value: value.value.clone(),
        }
    }
}

impl From<&JSONTopic> for Topic {
    fn from(topic: &JSONTopic) -> Self {
        let (folder, base) = util::split_name(&topic.name);
        let unit = if topic.unit.is_empty() {
            ::UNITLESS.to_string()
        } else {
            topic.unit.clone()
        };

        let attrs: Vec<Attribute> = topic.get_attrs();

        Topic {
            name: topic.name.clone(),
            name_base: base,
            name_folder: folder,
            unit,
            attrs,
            data: Vec::new(),
        }
    }
}

impl<'a> From<Topic> for Log {
    fn from(topic: Topic) -> Self {
        let (folder, base) = util::split_name(&topic.name);
        let unit = if topic.unit.is_empty() {
            ::UNITLESS.to_string()
        } else {
            topic.unit.clone()
        };

        Log {
            name: topic.name.clone(),
            name_base: base,
            name_folder: folder,
            unit,
            attrs: topic.attrs,
            data: Vec::new(),
            lines: Option::None,
        }
    }
}

impl From<&(String, Vec<String>)> for Topic {
    fn from(column: &(String, Vec<String>)) -> Self {
        let (folder, base) = util::split_name(&column.0);
        let unit = ::UNITLESS.to_string();

        let attrs = Vec::<Attribute>::new();

        Topic {
            name: column.0.clone(),
            name_base: base,
            name_folder: folder,
            unit,
            attrs,
            data: Vec::new(),
        }
    }
}

impl ParseMode {
    fn get_header(&self) -> Option<&JSONHeader> {
        match self {
            ParseMode::Bag(ref header) => Option::Some(header),
            ParseMode::Csv => Option::None,
        }
    }
}

impl Topic {
    fn fill(&mut self, data: &[String], trim_doubles: bool) {
        if self.attrs.len() == 1 && self.attrs[0].eq(&Attribute::Hide) {
            return;
        }

        for value in data {
            let mut datapoint = value.to_string().parse::<f64>();
            if datapoint.is_err() {
                if trim_doubles {
                    let test_value = value.trim();
                    datapoint = test_value.to_string().parse::<f64>();
                    if datapoint.is_err() {
                        error!("Failed to parse \"{}\" as a double", value);
                    }
                } else {
                    error!(
                        "Failed to parse \"{}\" as a double (maybe try --trim-doubles or hide topic)",
                        value
                    );
                }
            }
            let datapoint = datapoint.unwrap();
            self.data.push(datapoint);
        }
    }

    fn is_log(&self) -> bool {
        if self.attrs.contains(&Attribute::Log) {
            if self.attrs.len() > 1 {
                error!("Too many attributes on log topic {}", self.name)
            }
            true
        } else {
            false
        }
    }
}

impl Log {
    fn fill(&mut self, data: &[String], trim_doubles: bool) {
        for (i, value) in data.iter().enumerate() {
            let trimmed_value = if trim_doubles { value.trim() } else { value };

            if trimmed_value.parse::<f64>().is_err() {
                self.data.push((i as u64, value.to_string()));
            }
        }
    }

    pub fn apply_xaxis(&mut self, xaxis: &XAxis) {
        let mut lines = Vec::with_capacity(self.data.len());
        for line in &self.data {
            if line.1.is_empty() {
                continue;
            }

            if let Some(ref data) = xaxis.data {
                lines.push(format!(
                    "[{} {}] {}",
                    data[line.0 as usize], xaxis.unit, line.1
                ))
            } else {
                lines.push(format!("[{}] {}", line.0, line.1))
            }
        }
        self.lines = Some(lines);
    }
}

impl JSONTopic {
    fn get_attrs(&self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        for attr_text in &self.attrs {
            let attr = Attribute::from(&attr_text);
            if attr.is_err() {
                warning!("Failed to parse attribute {}, skipping it", attr_text);
                continue;
            }
            let attr = attr.unwrap();
            if attrs.contains(&attr) {
                warning!(
                    "Duplicate attribute \"{}\" on topic {}, ignoring duplicate",
                    attr_text,
                    self.name
                );
                continue;
            }
            attrs.push(attr);
        }
        attrs
    }
}

impl JSONHeader {
    fn get_values(&self) -> Vec<Value> {
        let mut values: Vec<Value> = Vec::new();
        for value in &self.values {
            if let Some(duplicate) = values.iter().find(|v| v.name.eq(&value.name)) {
                if !duplicate.value.eq(&value.value) {
                    error!("Duplicate value {} with different values", value.name);
                } else {
                    warning!("Duplicate value {}, ignoring duplicate", value.name);
                    continue;
                }
            }
            values.push(Value::from(value));
        }
        values
    }

    fn get_stream_shells(&self) -> (Vec<Topic>, Vec<Log>) {
        let mut topics: Vec<Topic> = Vec::new();
        let mut logs: Vec<Log> = Vec::new();
        for topic in &self.topics {
            if topics.iter().filter(|g| g.name.eq(&topic.name)).count() > 0 {
                error!("Duplicate topic entry in JSON header for {}", &topic.name);
            }

            let topic = Topic::from(topic);
            if topic.is_log() {
                logs.push(Log::from(topic));
            } else {
                topics.push(topic);
            }
        }

        (topics, logs)
    }
}

pub fn column_wise_csv_parse(mut reader: csv::Reader<File>) -> Vec<(String, Vec<String>)> {
    let csv_header = { reader.headers().unwrap().clone() };

    let mut output = Vec::with_capacity(csv_header.len());

    for i in 0..csv_header.len() {
        let column: String = (&csv_header[i]).to_string();
        output.push((column, Vec::new()));
    }

    for row in reader.records() {
        let row = match row {
            Err(e) => error!("{}", e.to_string()),
            Ok(row) => row,
        };

        if row.len() != output.len() {
            error!(
                "Row length ({}) does not match CSV header length ({})",
                row.len(),
                output.len()
            );
        }

        for i in 0..output.len() {
            let column: &mut (String, Vec<String>) = &mut output[i];
            column.1.push((&row[i]).to_string());
        }
    }

    output
}

fn parse_mid_input(input_path: &str, opt: &Opt) -> MidLevelInput {
    let input_file_contents: String = match fs::read_to_string(input_path) {
        Ok(contents) => contents,
        Err(f) => error!("Failed to open file \"{}\": {}", input_path, f.to_string()),
    };

    let (parse_mode, csv_text, json_header_text) = if opt.csv {
        (ParseMode::Csv, input_file_contents, Option::None)
    } else {
        let mut parts: Vec<&str> = input_file_contents.split('\n').collect();

        let json_header_text = parts.remove(0).to_string();
        let csv_text = parts.join("\n");

        let json_header = serde_json::from_str(&json_header_text);
        match json_header {
            Err(e) => error!(
                "Failed to parse json header: {} (if its a CSV file use --csv)",
                e.to_string()
            ),
            Ok(json_header) => (
                ParseMode::Bag(json_header),
                csv_text,
                Option::Some(json_header_text),
            ),
        }
    };

    let csv_reader: csv::Reader<File> = if opt.csv {
        csv::Reader::from_path(input_path).unwrap()
    } else {
        let mut tempfile = tempfile::tempfile().unwrap();
        tempfile.write_all(csv_text.as_bytes()).unwrap();
        tempfile.seek(SeekFrom::Start(0)).unwrap();
        csv::Reader::from_reader(tempfile)
    };

    MidLevelInput {
        json_header: parse_mode.get_header().cloned(),
        body: column_wise_csv_parse(csv_reader),
        json_header_text,
        csv_text,
    }
}

pub fn parse_input(input_path: &str, opt: &Opt) -> Input {
    let mid_input = parse_mid_input(input_path, opt);

    let (values, topics, logs) = if let Some(ref json_header) = mid_input.json_header {
        let (mut empty_topics, mut empty_logs) = json_header.get_stream_shells();
        for empty_topic in &mut empty_topics {
            match mid_input
                .body
                .iter()
                .filter(|x| (&(x.0)).eq(&(empty_topic.name)))
                .count()
            {
                0 => error!("Can't find topic \"{}\" in CSV", empty_topic.name),
                1 => {
                    let data = &mid_input
                        .body
                        .iter()
                        .filter(|x| (&(x.0)).eq(&(empty_topic.name)))
                        .last()
                        .unwrap()
                        .1;
                    empty_topic.fill(data, opt.trim_doubles);
                }
                _ => error!("Multiple columns \"{}\" found in CSV", empty_topic.name),
            }
        }

        for empty_log in &mut empty_logs {
            match mid_input
                .body
                .iter()
                .filter(|x| (&(x.0)).eq(&(empty_log.name)))
                .count()
            {
                0 => error!("Can't find topic \"{}\" in CSV", empty_log.name),
                1 => {
                    let data = &mid_input
                        .body
                        .iter()
                        .filter(|x| (&(x.0)).eq(&(empty_log.name)))
                        .last()
                        .unwrap()
                        .1;
                    empty_log.fill(data, opt.trim_doubles);
                }
                _ => error!("Multiple columns \"{}\" found in CSV", empty_log.name),
            }
        }
        (json_header.get_values(), empty_topics, empty_logs)
    } else {
        let topics: Vec<Topic> = mid_input
            .body
            .iter()
            .map(|x| {
                let mut topic = Topic::from(x);
                topic.fill(&x.1, opt.trim_doubles);
                topic
            })
            .collect();
        (Vec::new(), topics, Vec::new())
    };

    Input {
        topics,
        logs,
        values,
        json_header_text: mid_input.json_header_text,
        csv_text: mid_input.csv_text,
    }
}
