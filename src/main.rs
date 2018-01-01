extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;
extern crate csv;

use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "badlogvis", about = "Create html from badlog data")]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,

    #[structopt(help = "Output file, default to <input>.html")]
    output: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Topic {
    name: String,
    unit: String,
    attrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Value {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONHeader {
    topics: Vec<Topic>,
    values: Vec<Value>,
}

#[derive(Debug)]
struct Graph {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: String,
    pub attrs: Vec<String>,
    pub data: Vec<(f64, f64)>,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input = opt.input;

    let contents: String = {
        let mut f = File::open(input).expect("file not found");
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();
        contents
    };

    let json_header = contents.lines().take(1).last().unwrap().to_string();
    let csv_data = contents.lines().skip(1).fold("".to_string(), |a, b| {
        if a.len() == 0 {
            b.to_string()
        } else {
            [a, b.to_string()].join("\n")
        }
    });

    let p: JSONHeader = serde_json::from_str(&json_header).unwrap();

    let mut tempfile = tempfile::tempfile().unwrap();
    tempfile.write(csv_data.as_bytes()).unwrap();
    tempfile.seek(SeekFrom::Start(0)).unwrap();

    let rdr = csv::Reader::from_reader(tempfile);

    let graphs = gen_graphs(p, rdr);

    println!("{:?}", graphs);

    graphs.iter().for_each(|g| println!("{}", g.gen_highchart()));
}

fn split_name(name: &str) -> (String, String) {
    let mut parts: Vec<&str> = name.split("/").collect();

    assert!(parts.len() > 0);

    if parts.len() == 1 {
        return ("".to_string(), parts[0].to_string());
    }

    let base = parts.pop().unwrap().to_string();
    let folder = parts.join("/");

    (folder, base)
}

fn gen_graphs(json_header: JSONHeader, mut csv_reader: csv::Reader<File>) -> Vec<Graph> {
    let mut out: Vec<Graph> = Vec::new();

    for topic in json_header.topics {
        let (folder, base) = split_name(&topic.name);
        let mut graph = Graph {
            name: topic.name,
            name_base: base,
            name_folder: folder,
            unit: topic.unit,
            attrs: topic.attrs,
            data: Vec::new(),
        };
        out.push(graph);
    }

    let header = {
        csv_reader.headers().unwrap().clone()
    };

    let mut step = 0;
    for row in csv_reader.records() {
        let row = row.unwrap();
        assert_eq!(row.len(), header.len());

        for i in 0..header.len() {
            let (k, v) = (&header[i], &row[i]);

            assert_eq!(out.iter().filter(|g| g.name.eq(k)).count(), 1);
            let graph = out.iter_mut().filter(|g| g.name.eq(k)).last().unwrap();
            graph.data.push(((step as f64), v.to_string().parse::<f64>().unwrap()));
        }

        step += 1;
    }

    out
}

impl Graph {
    pub fn gen_highchart(&self) -> String {
        let data: String = self.data.iter().map(|p| {
            let (x, y) = *p;
            format!("[{},{}]", x, y)
        }).fold("".to_string(), |a, b| {
            if a.len() == 0 {
                b.to_string()
            } else {
                [a, b.to_string()].join(",")
            }
        });
        format!("\
<div id=\"{name}\" style=\"min-width: 310px; height: 400px; margin: 0 auto\"></div>
<script>
    Highcharts.chart('{name}', {{
        chart: {{
            type: 'line'
        }},
        title: {{
            text: '{title}'
        }},
        subtitle: {{
            text: '{name}'
        }},
        yAxis: {{
            title: {{
                text: '{unit}'
            }}
        }},
        series: [{{
            //name: '{title}',
            data: [{data}]
        }}]
    }});
</script>\
", name = self.name, unit = self.unit, title = self.name_base, data = data)
    }
}