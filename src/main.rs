extern crate base64;
extern crate colored;
extern crate csv;
extern crate flate2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate tempfile;

#[macro_use]
mod util;
mod attribute;
mod folder;
mod graph;
mod input;

use std::fs::File;
use std::io::prelude::*;

use structopt::StructOpt;

use folder::Folder;
use graph::Graph;
use input::*;

pub const UNITLESS: &str = "ul";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(StructOpt, Debug)]
#[structopt(name = "badlogvis", about = "Create html from badlog data")]
pub struct Opt {
    #[structopt(help = "Input file")]
    input: String,

    #[structopt(help = "Output file, default to <input>.html")]
    output: Option<String>,

    #[structopt(short = "t", long = "trim-doubles",
                help = "Retry parsing doubles without whitespace")]
    trim_doubles: bool,

    #[structopt(short = "c", long = "csv", help = "Input is CSV file")]
    csv: bool,

    #[structopt(short = "g", long = "gzip", help = "Compress embeded CSV file")]
    compress_csv: bool,
}

enum CsvEmbed {
    Raw(String),
    Compressed(Vec<u8>),
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input_path = opt.input.clone();
    let output = opt.output
        .clone()
        .unwrap_or_else(|| format!("{}.html", input_path));

    let input = parse_input(&input_path, &opt);

    let graphs = Graph::gen_graphs(&input.topics);

    let folders: Vec<Folder> = Folder::gen_folders(graphs, input.values);

    let csv_embed = if opt.compress_csv {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::prelude::*;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&input.csv_text.as_bytes()).unwrap();
        CsvEmbed::Compressed(encoder.finish().unwrap())
    } else {
        CsvEmbed::Raw(input.csv_text)
    };

    let out = gen_html(
        &input_path,
        folders,
        &csv_embed,
        input.json_header.as_ref().map(String::as_str),
    );

    let mut outfile = File::create(output).unwrap();
    outfile.write_all(out.as_bytes()).unwrap();
}

fn gen_html(
    input: &str,
    folders: Vec<Folder>,
    csv_embed: &CsvEmbed,
    json_header: Option<&str>,
) -> String {
    let bootstrap_css_source = include_str!("web_res/bootstrap.min.css");
    let jquery_js_source = include_str!("web_res/jquery-3.2.1.min.js");
    let bootstrap_js_source = include_str!("web_res/bootstrap.min.js");
    let highcharts_js_source = include_str!("web_res/highcharts.js");
    let highcharts_boost_js_source = include_str!("web_res/boost.js");
    let highcharts_exporting_js_source = include_str!("web_res/exporting.js");
    let highcharts_offline_exporting_source = include_str!("web_res/offline-exporting.js");

    let (csv_base64, extention) = match csv_embed {
        CsvEmbed::Raw(ref csv_raw) => (base64::encode(csv_raw), "csv"),
        CsvEmbed::Compressed(ref data) => (base64::encode(data), "csv.gz"),
    };
    let csv_filename = format!("{}.{}", input, extention);

    let mut content = String::new();

    for folder in folders {
        content += &folder.gen_html();
    }

    let json_header = if let Some(header) = json_header {
        format!(r#"<div class="well">{}</div>"#, header)
    } else {
        "".to_string()
    };

    format!(r##"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>BadLog - {title}</title>

    <!-- bootstrap.min.css -->
    <style type="text/css">
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
    <div class="container">
      <div class="page-header">
        <h1>{title} <a href="data:text/csv;base64,{csv_base64}" download="{csv_filename}" class="btn btn-default btn-md">Download {extention}</a></h1>
      </div>

      {content}

      <a style="color: grey; text-decoration: underline;" data-toggle="collapse" href="#metadata" aria-expanded="false" aria-controls="metadata">Info</a>
      <div class="collapse" id="metadata">
        {json_header}
        <p>badlogvis {badlogvis_version}</p>
      </div>
    </div> <!-- /container -->
  </body>
</html>"##, title = input, bootstrap_css = bootstrap_css_source, jquery_js = jquery_js_source, bootstrap_js = bootstrap_js_source,
            highcharts_js = highcharts_js_source, boost_js = highcharts_boost_js_source,
            content = content, csv_base64 = csv_base64, csv_filename = csv_filename,
            exporting_js = highcharts_exporting_js_source,
            offline_exporting_js = highcharts_offline_exporting_source,
            extention = extention.to_string().to_uppercase(),
            badlogvis_version = VERSION, json_header = json_header)
}
