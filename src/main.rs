extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tempfile;
extern crate csv;
extern crate base64;
extern crate colored;

use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

use structopt::StructOpt;

use std::cmp::Ordering::Equal;

use colored::*;

const UNITLESS: &str = "ul";

macro_rules! error {
    ($fmt:expr) => {
        println!(concat!("{}: ", $fmt), "error".bold().red());
        std::process::exit(1);
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!(concat!("{}: ", $fmt), "error".bold().red(), $($arg)*);
        std::process::exit(1);
    };
}

macro_rules! warning {
    ($fmt:expr) => {
        println!(concat!("{}: ", $fmt), "warning".bold().yellow());
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!(concat!("{}: ", $fmt), "warning".bold().yellow(), $($arg)*);
    };
}

#[derive(StructOpt, Debug)]
#[structopt(name = "badlogvis", about = "Create html from badlog data")]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,

    #[structopt(help = "Output file, default to <input>.html")]
    output: Option<String>,

    #[structopt(short = "t", long = "trim-doubles", help = "Retry parsing doubles without whitespace")]
    trim_doubles: bool,

    #[structopt(short = "c", long = "csv", help = "Input is CSV file")]
    csv: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONTopic {
    name: String,
    unit: String,
    attrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONValue {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONHeader {
    topics: Vec<JSONTopic>,
    values: Vec<JSONValue>,
}

#[derive(Debug)]
struct Topic {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: Option<String>,
    pub attrs: Vec<String>,
    pub data: Vec<(f64, f64)>,
}

#[derive(Debug)]
struct Value {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub value: String,
}

#[derive(Debug)]
struct Folder {
    pub name: String,
    pub table: Vec<Value>,
    pub topics: Vec<Topic>,
}

#[derive(Debug)]
enum ParseMode {
    Bag(JSONHeader),
    Csv,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input = opt.input;
    let output = opt.output.unwrap_or(format!("{}.html", input));

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

    let rdr: csv::Reader<File> = if opt.csv {
        csv::Reader::from_path(&input).unwrap()
    } else {
        let mut tempfile = tempfile::tempfile().unwrap();
        tempfile.write(csv_text.as_bytes()).unwrap();
        tempfile.seek(SeekFrom::Start(0)).unwrap();
        csv::Reader::from_reader(tempfile)
    };

    let folders: Vec<Folder> = gen_folders(parse_mode, rdr, opt.trim_doubles);

    let out = gen_html(&input, folders, &csv_text);

    let mut outfile = File::create(output).unwrap();
    outfile.write_all(out.as_bytes()).unwrap();
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

fn gen_folders(parse_mode: ParseMode, mut csv_reader: csv::Reader<File>, trim_doubles: bool) -> Vec<Folder> {
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
                    let (folder, base) = split_name(&topic.name);
                    let unit = if topic.unit.len() == 0 || topic.unit.eq(UNITLESS) {
                        Option::None
                    } else {
                        Option::Some(topic.unit.clone())
                    };
                    let mut topic = Topic {
                        name: topic.name.clone(),
                        name_base: base,
                        name_folder: folder,
                        unit,
                        attrs: topic.attrs.clone(),
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
                    let (folder, base) = split_name(&name);
                    let mut topic = Topic {
                        name,
                        name_base: base,
                        name_folder: folder,
                        unit: Option::None,
                        attrs: Vec::new(),
                        data: Vec::new(),
                    };
                    topics.push(topic);
                }
            }
        }

        let mut step: i32 = 0;
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
                topic.data.push(((step as f64), datapoint));
            }

            step += 1;
        }

        topics
    };

    let values = {
        let mut values = Vec::new();

        match parse_mode {
            ParseMode::Bag(json_header) => {
                for value in json_header.values {
                    let (folder, base) = split_name(&value.name);
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

    let mut folders: Vec<Folder> = Vec::new();

    'outer_topic: for topic in topics {
        for folder in folders.iter_mut() {
            if folder.name.eq(&topic.name_folder) {
                folder.topics.push(topic);
                continue 'outer_topic;
            }
        }
        folders.push(Folder {
            name: topic.name_folder.clone(),
            table: Vec::new(),
            topics: vec![topic],
        });
    }

    'outer_value: for value in values {
        for folder in folders.iter_mut() {
            if folder.name.eq(&value.name_folder) {
                folder.table.push(value);
                continue 'outer_value;
            }
        }
        folders.push(Folder {
            name: value.name_folder.clone(),
            table: vec![value],
            topics: Vec::new(),
        });
    }

    for folder in folders.iter_mut() {
//        folder.table.sort_by(|a, b| a.name_base.to_ascii_lowercase().cmp(&(b.name_base.to_ascii_lowercase())));
        folder.topics.sort_by(|a, b| a.name_base.to_ascii_lowercase().cmp(&b.name_base.to_ascii_lowercase()));
    }

    folders.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));

    folders
}

impl Topic {
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

        let min_y = self.data.iter().map(|p| {
            let (_, y) = *p;
            y
        }).min_by(|a, b| a.partial_cmp(b).unwrap_or(Equal)).unwrap();

        let min_y = if min_y < 0f64 { min_y } else { 0f64 };

        let unit = match &self.unit {
            &None => "".to_string(),
            &Some(ref unit) => format!(" ({})", unit)
        };

        format!("\
<div id=\"{name}\" style=\"min-width: 310px; height: 400px; margin: 0 auto\"></div>
<script>
    Highcharts.chart('{name}', {{
        chart: {{
            type: 'line',
            zoomType: 'x'
        }},
        title: {{
            text: '{title}{unit}'
        }},
        subtitle: {{
            text: '{name}'
        }},
        yAxis: {{
            min: {min_y}
        }},
        xAxis: {{
            events: {{
                setExtremes: syncExtremes
            }}
        }},
        credits: {{
            enabled: false
        }},
        series: [{{
            //name: '{title}',
            data: [{data}]
        }}]
    }});
</script>\
", name = self.name, unit = unit, title = self.name_base, data = data, min_y = min_y)
    }
}

impl Folder {
    pub fn gen_html(&self) -> String {
        let table = gen_table(&self.table);
        let mut graph_content = String::new();
        for topic in self.topics.iter() {
            graph_content += &topic.gen_highchart();
        }

        if self.name.len() == 0 {
            return format!("{table}\n{graphs}", table = table, graphs = graph_content);
        }

        format!("\
  <div class=\"panel-group\">
    <div class=\"panel panel-default\">
      <div class=\"panel-heading\">
        <h4 class=\"panel-title\">
          <a data-toggle=\"collapse\" href=\"#collapse_{name}\">{name}</a>
        </h4>
      </div>
      <div id=\"collapse_{name}\" class=\"panel-collapse collapse\">
        <div class=\"panel-body\">
          {table}
          {graphs}
        </div>
      </div>
    </div>
  </div>", name = self.name, table = table, graphs = graph_content)
    }
}

fn gen_table(values: &Vec<Value>) -> String {
    if values.len() == 0 {
        return "<!-- Empty table omitted -->\n".to_string();
    }
    let mut rows = String::new();
    for value in values.iter() {
        rows += &format!("<tr><td>{name}</td><td>{value}</td></tr>\n", name = value.name_base, value = value.value);
    }
    format!("<table class=\"table table-striped\"><thead><tr><th>Name</th><th>Value</th></tr></thead><tbody>\n{rows}</tbody></table>\n", rows = rows)
}

fn gen_html(input: &str, folders: Vec<Folder>, csv_raw: &str) -> String {
    let bootstrap_css_source = include_str!("web_res/bootstrap.min.css");
    let jquery_js_source = include_str!("web_res/jquery-3.2.1.min.js");
    let bootstrap_js_source = include_str!("web_res/bootstrap.min.js");
    let highcharts_js_source = include_str!("web_res/highcharts.js");
    let highcharts_boost_js_source = include_str!("web_res/boost.js");
    let highcharts_exporting_js_source = include_str!("web_res/exporting.js");
    let highcharts_offline_exporting_source = include_str!("web_res/offline-exporting.js");

    let csv_base64 = base64::encode(csv_raw);
    let csv_filename = format!("{}.csv", input);

    let mut content = String::new();

    for folder in folders {
        content += &folder.gen_html();
    }

    format!("\
<!DOCTYPE html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
    <title>BadLog - {title}</title>

    <!-- bootstrap.min.css -->
    <style type=\"text/css\">
        {bootstrap_css}
    </style>

    <!-- jquery-3.2.1.min.js -->
    <script>
        {jquery_js}
    </script>

    <!-- bootstrap.min.js -->
    <script>
        {bootstrap_js}
    </script>

    <!-- highcharts.js -->
    <script>
        {highcharts_js}
    </script>

    <!-- boost.js -->
    <script>
        {boost_js}
    </script>

    <!-- exporting.js -->
    <script>
        {exporting_js}
    </script>

    <!-- offline-exporting.js -->
    <script>
        {offline_exporting_js}
    </script>

    <!-- For syncronizing chart zooms -->
    <script>
        function syncExtremes(e) {{
            var thisChart = this.chart;

            if (e.trigger !== 'syncExtremes') {{ // Prevent feedback loop
                Highcharts.each(Highcharts.charts, function (chart) {{
                    if (chart !== thisChart) {{
                        if (chart.xAxis[0].setExtremes) {{ // It is null while updating
                            chart.xAxis[0].setExtremes(e.min, e.max, undefined, false, {{ trigger: 'syncExtremes' }});
                        }}
                    }}
                }});
            }}
        }}
    </script>

  </head>

  <body>
    <div class=\"container\">
      <div class=\"page-header\">
        <h1>{title} <a href=\"data:text/csv;base64,{csv_base64}\" download=\"{csv_filename}\" class=\"btn btn-default btn-md\">Download CSV</a></h1>
      </div>

      {content}
    </div> <!-- /container -->
  </body>
</html>
\
    ", title = input, bootstrap_css = bootstrap_css_source, jquery_js = jquery_js_source, bootstrap_js = bootstrap_js_source,
            highcharts_js = highcharts_js_source, boost_js = highcharts_boost_js_source, content = content,
            csv_base64 = csv_base64, csv_filename = csv_filename, exporting_js = highcharts_exporting_js_source,
            offline_exporting_js = highcharts_offline_exporting_source)
}