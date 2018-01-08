use attribute::Attribute;

use util;
use csv;

use ::Opt;

use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use serde_json;
use tempfile;

#[derive(Serialize, Deserialize, Debug)]
pub struct JSONTopic {
    pub name: String,
    pub unit: String,
    pub attrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JSONValue {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JSONHeader {
    pub topics: Vec<JSONTopic>,
    pub  values: Vec<JSONValue>,
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
pub enum ParseMode {
    Bag(JSONHeader),
    Csv,
}

pub fn parse_input(input: &str, opt: &Opt) -> (Vec<Topic>, Vec<Value>, String) {
    let input = input.to_string();
    let input_file_contents: String = {
        let mut f = File::open(input.clone());
        if f.is_err() {
            error!("Failed to open file \"{}\": {}", input, f.unwrap_err().to_string());
        }
        let mut f = f.unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        contents
    };

    let (parse_mode, csv_text) = if opt.csv {
        (ParseMode::Csv, input_file_contents)
    } else {
        let csv_text =
            input_file_contents.lines().skip(1).fold("".to_string(), |a, b| {
                if a.len() == 0 {
                    b.to_string()
                } else {
                    [a, b.to_string()].join("\n")
                }
            });

        let json_header_text = input_file_contents.lines().take(1).last().unwrap().to_string(); // First line
        let json_header = serde_json::from_str(&json_header_text);
        if json_header.is_err() {
            error!("Failed to parse json header: {} (if its a CSV file use --csv)", json_header.unwrap_err().to_string());
        }
        let json_header = json_header.unwrap();
        (ParseMode::Bag(json_header), csv_text)
    };

    let mut csv_reader: csv::Reader<File> = if opt.csv {
        csv::Reader::from_path(&input).unwrap()
    } else {
        let mut tempfile = tempfile::tempfile().unwrap();
        tempfile.write(csv_text.as_bytes()).unwrap();
        tempfile.seek(SeekFrom::Start(0)).unwrap();
        csv::Reader::from_reader(tempfile)
    };

    let trim_doubles = opt.trim_doubles;
    let topics = {
        let mut topics: Vec<Topic> = Vec::new();

        let header = {
            csv_reader.headers().unwrap().clone()
        };

        match &parse_mode {
            &ParseMode::Bag(ref json_header) => {
                for topic in json_header.topics.iter() {
                    if topics.iter().filter(|g| g.name.eq(&topic.name)).count() > 0 {
                        error!("Duplicate topic entry in JSON header for {}", &topic.name);
                    }
                    let (folder, base) = util::split_name(&topic.name);
                    let unit = if topic.unit.len() == 0 {
                        ::UNITLESS.to_string()
                    } else {
                        topic.unit.clone()
                    };

                    let attrs: Vec<Attribute> = {
                        let mut attrs = Vec::new();
                        for attr_text in topic.attrs.iter() {
                            let attr = Attribute::from(&attr_text);
                            if attr.is_err() {
                                warning!("Failed to parse attribute {}, skipping it", attr_text);
                                continue;
                            }
                            let attr = attr.unwrap();
                            if attrs.contains(&attr) {
                                warning!("Duplicate attribute \"{}\" on topic {}, ignoring duplicate", attr_text, topic.name);
                                continue;
                            }
                            attrs.push(attr);
                        }
                        attrs
                    };

                    let mut topic = Topic {
                        name: topic.name.clone(),
                        name_base: base,
                        name_folder: folder,
                        unit,
                        attrs,
                        data: Vec::new(),
                    };
                    topics.push(topic);
                }
            }
            &ParseMode::Csv => {
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
            }
        }

        for row in csv_reader.records() {
            if row.is_err() {
                error!("{}", &row.unwrap_err().to_string());
            }
            let row = row.unwrap();

            for i in 0..header.len() {
                let (k, v) = (&header[i], &row[i]);

                {
                    let count = topics.iter().filter(|g| g.name.eq(k)).count();
                    if count != 1 {
                        if count > 1 {
                            panic!();
                        }
                        error!("Can't find topic {} in JSON header", k);
                    }
                }

                let topic = topics.iter_mut().filter(|g| g.name.eq(k)).last().unwrap();
                let mut datapoint = v.to_string().parse::<f64>();
                if datapoint.is_err() {
                    if trim_doubles {
                        let test_v = v.trim();
                        datapoint = test_v.to_string().parse::<f64>();
                        if datapoint.is_err() {
                            error!("Failed to parse \"{}\" as a double", v);
                        }
                    } else {
                        error!("Failed to parse \"{}\" as a double (maybe try --trim-doubles)", v);
                    }
                }
                let datapoint = datapoint.unwrap();
                topic.data.push(datapoint);
            }
        }

        topics
    };

    let values = {
        let mut values = Vec::new();

        match parse_mode {
            ParseMode::Bag(json_header) => {
                for value in json_header.values {
                    let (folder, base) = util::split_name(&value.name);
                    values.push(Value {
                        name: value.name,
                        name_base: base,
                        name_folder: folder,
                        value: value.value,
                    });
                }
            }
            ParseMode::Csv => {}
        }

        values
    };

    (topics, values, csv_text)
}