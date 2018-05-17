use attribute::Attribute;

use csv;
use util;

use Opt;

use serde_json;
use std::convert::From;
use std::fs::{self, File};
use std::io::SeekFrom;
use std::io::prelude::*;
use tempfile;

#[derive(Serialize, Deserialize, Debug)]
struct JSONTopic {
    pub name: String,
    pub unit: String,
    pub attrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONValue {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONHeader {
    pub topics: Vec<JSONTopic>,
    pub values: Vec<JSONValue>,
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
pub struct Value {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub value: String,
}

#[derive(Debug)]
pub struct Input {
    pub topics: Vec<Topic>,
    pub values: Vec<Value>,
    pub json_header: Option<String>,
    pub csv_text: String,
}

impl<'a> From<&'a JSONValue> for Value {
    fn from(value: &'a JSONValue) -> Self {
        let (folder, base) = util::split_name(&value.name);
        Value {
            name: value.name.clone(),
            name_base: base,
            name_folder: folder,
            value: value.value.clone(),
        }
    }
}

impl<'a> From<&'a JSONTopic> for Topic {
    fn from(topic: &'a JSONTopic) -> Self {
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

#[derive(Debug)]
enum ParseMode {
    Bag(JSONHeader),
    Csv,
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

    fn get_topics(&self) -> Vec<Topic> {
        let mut topics: Vec<Topic> = Vec::new();
        for topic in &self.topics {
            if topics.iter().filter(|g| g.name.eq(&topic.name)).count() > 0 {
                error!("Duplicate topic entry in JSON header for {}", &topic.name);
            }
            topics.push(Topic::from(topic));
        }
        topics
    }
}

fn get_topics_from_csv(header: &csv::StringRecord) -> Vec<Topic> {
    let mut topics: Vec<Topic> = Vec::new();
    for topic in header.iter() {
        let name = topic.to_string();

        if name.ne(&name.trim()) {
            warning!("Topic \"{}\" has exterior whitespace", name);
        }

        if topics.iter().filter(|g| g.name.eq(&name)).count() > 0 {
            error!("Duplicate topic entry in CSV header for {}", name);
        }
        let (folder, base) = util::split_name(&name);
        let mut topic = Topic {
            name,
            name_base: base,
            name_folder: folder,
            unit: ::UNITLESS.to_string(),
            attrs: Vec::new(),
            data: Vec::new(),
        };
        topics.push(topic);
    }
    topics
}

pub fn parse_input(input_path: &str, opt: &Opt) -> Input {
    let input_file_contents: String = match fs::read_to_string(input_path) {
        Ok(contents) => contents,
        Err(f) => error!("Failed to open file \"{}\": {}", input_path, f.to_string()),
    };

    let (parse_mode, csv_text, json_header) = if opt.csv {
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

    let mut csv_reader: csv::Reader<File> = if opt.csv {
        csv::Reader::from_path(input_path).unwrap()
    } else {
        let mut tempfile = tempfile::tempfile().unwrap();
        tempfile.write_all(csv_text.as_bytes()).unwrap();
        tempfile.seek(SeekFrom::Start(0)).unwrap();
        csv::Reader::from_reader(tempfile)
    };

    let trim_doubles = opt.trim_doubles;
    let topics = {
        let csv_header = { csv_reader.headers().unwrap().clone() };

        let mut topics = match parse_mode {
            ParseMode::Bag(ref json_header) => json_header.get_topics(),
            ParseMode::Csv => get_topics_from_csv(&csv_header),
        };

        for row in csv_reader.records() {
            let row = match row {
                Err(e) => error!("{}", e.to_string()),
                Ok(row) => row,
            };

            for i in 0..csv_header.len() {
                let (k, v) = (&csv_header[i], &row[i]);

                let mut topic = match topics.iter_mut().filter(|g| g.name.eq(k)).last() {
                    Some(t) => t,
                    None => error!("Can't find topic {} in JSON header", k),
                };

                // Don't parse if topic hidden and no derived graphs
                if topic.attrs.len() == 1 && topic.attrs[0].eq(&Attribute::Hide) {
                    continue;
                }

                let mut datapoint = v.to_string().parse::<f64>();
                if datapoint.is_err() {
                    if trim_doubles {
                        let test_v = v.trim();
                        datapoint = test_v.to_string().parse::<f64>();
                        if datapoint.is_err() {
                            error!("Failed to parse \"{}\" as a double", v);
                        }
                    } else {
                        error!("Failed to parse \"{}\" as a double (maybe try --trim-doubles or hide topic)", v);
                    }
                }
                let datapoint = datapoint.unwrap();
                topic.data.push(datapoint);
            }
        }

        topics
    };

    let values = match parse_mode {
        ParseMode::Bag(json_header) => json_header.get_values(),
        ParseMode::Csv => Vec::new(),
    };

    Input {
        topics,
        values,
        json_header,
        csv_text,
    }
}
