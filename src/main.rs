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

#[derive(Debug)]
struct SortedValue {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub value: String,
}

#[derive(Debug)]
struct Folder {
    pub name: String,
    pub table: Vec<SortedValue>,
    pub graphs: Vec<Graph>,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input = opt.input;
    let output = opt.output.unwrap_or(format!("{}.html", input));

    let contents: String = {
        let mut f = File::open(input.clone());
        if f.is_err() {
            println!("{} Error opening file \"{}\": {}", "error:".bold().red(), input, f.unwrap_err().to_string());
            std::process::exit(1);
        }
        let mut f = f.unwrap();
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

    let folders: Vec<Folder> = gen_folders(p, rdr);

    let out = gen_html(&input, folders, &csv_data);

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

fn gen_folders(json_header: JSONHeader, mut csv_reader: csv::Reader<File>) -> Vec<Folder> {
    let graphs = {
        let mut graphs: Vec<Graph> = Vec::new();

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
            graphs.push(graph);
        }

        let header = {
            csv_reader.headers().unwrap().clone()
        };

        let mut step: i32 = 0;
        for row in csv_reader.records() {
            let row = row.unwrap();
            assert_eq!(row.len(), header.len());

            for i in 0..header.len() {
                let (k, v) = (&header[i], &row[i]);

                assert_eq!(graphs.iter().filter(|g| g.name.eq(k)).count(), 1);
                let graph = graphs.iter_mut().filter(|g| g.name.eq(k)).last().unwrap();
                let datapoint = v.to_string().parse::<f64>().expect(&format!("Failed to parse f64 : {:?} on line {}", v, step));
                graph.data.push(((step as f64), datapoint));
            }

            step += 1;
        }

        graphs
    };

    let values = {
        let mut values = Vec::new();

        for value in json_header.values {
            let (folder, base) = split_name(&value.name);
            values.push(SortedValue {
                name: value.name,
                name_base: base,
                name_folder: folder,
                value: value.value,
            });
        }

        values
    };

    let mut folders: Vec<Folder> = Vec::new();

    'outer_graph: for graph in graphs {
        for folder in folders.iter_mut() {
            if folder.name.eq(&graph.name_folder) {
                folder.graphs.push(graph);
                continue 'outer_graph;
            }
        }
        folders.push(Folder {
            name: graph.name_folder.clone(),
            table: Vec::new(),
            graphs: vec![graph],
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
            graphs: Vec::new(),
        });
    }

    for folder in folders.iter_mut() {
//        folder.table.sort_by(|a, b| a.name_base.to_ascii_lowercase().cmp(&(b.name_base.to_ascii_lowercase())));
        folder.graphs.sort_by(|a, b| a.name_base.to_ascii_lowercase().cmp(&b.name_base.to_ascii_lowercase()));
    }

    folders.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));

    folders
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

        let min_y = self.data.iter().map(|p| {
            let (_, y) = *p;
            y
        }).min_by(|a, b| a.partial_cmp(b).unwrap_or(Equal)).unwrap();

        let min_y = if min_y < 0f64 { min_y } else { 0f64 };

        format!("\
<div id=\"{name}\" style=\"min-width: 310px; height: 400px; margin: 0 auto\"></div>
<script>
    Highcharts.chart('{name}', {{
        chart: {{
            type: 'line',
            zoomType: 'x'
        }},
        title: {{
            text: '{title} ({unit})'
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
", name = self.name, unit = self.unit, title = self.name_base, data = data, min_y = min_y)
    }
}

impl Folder {
    pub fn gen_html(&self) -> String {
        let table = gen_table(&self.table);
        let mut graph_content = String::new();
        for graph in self.graphs.iter() {
            graph_content += &graph.gen_highchart();
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

fn gen_table(values: &Vec<SortedValue>) -> String {
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