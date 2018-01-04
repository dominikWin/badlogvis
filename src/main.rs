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

#[macro_use]
mod util;
mod attribute;
mod graph;
mod input;

use std::fs::File;
use std::io::prelude::*;

use structopt::StructOpt;

use attribute::Attribute;
use graph::{Graph, Series};
use input::*;

use colored::*;


pub const UNITLESS: &str = "ul";

#[derive(StructOpt, Debug)]
#[structopt(name = "badlogvis", about = "Create html from badlog data")]
pub struct Opt {
    #[structopt(help = "Input file")]
    input: String,

    #[structopt(help = "Output file, default to <input>.html")]
    output: Option<String>,

    #[structopt(short = "t", long = "trim-doubles", help = "Retry parsing doubles without whitespace")]
    trim_doubles: bool,

    #[structopt(short = "c", long = "csv", help = "Input is CSV file")]
    csv: bool,
}

#[derive(Debug)]
struct Folder {
    pub name: String,
    pub table: Vec<Value>,
    pub graphs: Vec<Graph>,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input = opt.input.clone();
    let output = opt.output.clone().unwrap_or(format!("{}.html", input));

    let (topics, values, csv_text) = parse_input(&input, &opt);

    let graphs = gen_graphs(topics);

    let folders: Vec<Folder> = gen_folders(graphs, values);

    let out = gen_html(&input, folders, &csv_text);

    let mut outfile = File::create(output).unwrap();
    outfile.write_all(out.as_bytes()).unwrap();
}

fn gen_graphs(topics: Vec<Topic>) -> Vec<Graph> {
    let xaxis_index: Option<usize> = {
        let mut out = Option::None;
        for i in 0..topics.len() {
            let topic: &Topic = &topics[i];
            if topic.attrs.contains(&Attribute::Xaxis) {
                if out.is_some() {
                    error!("Multiple topics with xaxis attribute");
                } else {
                    out = Some(i);
                }
            }
        }
        out
    };

    let x_unit = if let Some(x_index) = xaxis_index {
        let unit_text = match topics[x_index].unit.clone() {
            None => "".to_string(),
            Some(unit) => format!(" ({})", unit),
        };
        (unit_text)
    } else {
        "Index".to_string()
    };

    let mut graphs = Vec::new();
    for i in 0..topics.len() {
        let topic: &Topic = &topics[i];

        {
            if topic.attrs.contains(&Attribute::Differentiate) {
                let name = format!("{} Derivative", topic.name);
                let (name_folder, name_base) = util::split_name(&name);
                let unit: Option<String> = Option::None;
                let data = if let Some(x_index) = xaxis_index {
                    let x_data = topics[x_index].data.clone();
                    util::bind_axis(x_data, topic.data.clone())
                } else {
                    util::fake_x_axis(topic.data.clone())
                };
                let data = util::differention(&data);
                let series: Series = Series {
                    name: Option::None,
                    data,
                };
                let area = false;
                let direct = false;

                graphs.push(Graph {
                    name,
                    name_base,
                    name_folder,
                    unit,
                    x_unit: x_unit.clone(),
                    series: vec![series],
                    area,
                    direct,
                })
            }
        }

        if topic.attrs.contains(&Attribute::Hide) {
            continue;
        }

        let area = topic.attrs.contains(&Attribute::Area);

        let data = if let Some(x_index) = xaxis_index {
            let x_data = topics[x_index].data.clone();
            util::bind_axis(x_data, topic.data.clone())
        } else {
            util::fake_x_axis(topic.data.clone())
        };

        let series: Series = Series {
            name: Option::None,
            data,
        };

        graphs.push(Graph {
            name: topic.name.clone(),
            name_base: topic.name_base.clone(),
            name_folder: topic.name_folder.clone(),
            unit: topic.unit.clone(),
            x_unit: x_unit.clone(),
            series: vec![series],
            area,
            direct: true,
        });
    }
    graphs
}

fn gen_folders(graphs: Vec<Graph>, values: Vec<Value>) -> Vec<Folder> {
    let mut folders: Vec<Folder> = Vec::new();

    'outer_topic: for graph in graphs {
        for folder in folders.iter_mut() {
            if folder.name.eq(&graph.name_folder) {
                folder.graphs.push(graph);
                continue 'outer_topic;
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

impl Folder {
    pub fn gen_html(&self) -> String {
        let table = gen_table(&self.table);
        let mut graph_content = String::new();
        for topic in self.graphs.iter() {
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