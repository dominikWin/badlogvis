use std::cmp::Ordering::Equal;
use util;

#[derive(Debug)]
pub struct Graph {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: Option<String>,
    pub x_unit: String,
    pub series: Vec<Series>,
    pub area: bool,
    pub direct: bool,
}

#[derive(Debug)]
pub struct Series {
    pub name: Option<String>,
    pub data: Vec<(f64, f64)>,
}

impl Graph {
    pub fn from_default(name: String, unit: Option<String>, x_unit: String, series: Vec<Series>, direct: bool) -> Graph {
        let (name_folder, name_base) = util::split_name(&name);
        Graph {
            name,
            name_base,
            name_folder,
            unit,
            x_unit,
            series,
            area: false,
            direct,
        }
    }

    pub fn gen_highchart(&self) -> String {
        let mut series_content = String::new();
        let mut min_y = 0f64;
        for s in self.series.iter() {
            let data: String = s.data.iter().map(|p| {
                let (x, y) = *p;
                format!("[{},{}]", x, y)
            }).fold("".to_string(), |a, b| {
                if a.len() == 0 {
                    b.to_string()
                } else {
                    [a, b.to_string()].join(",")
                }
            });
            let name = if let Some(name) = s.name.clone() {
                format!("name: '{}',", name)
            } else {
                "".to_string()
            };
            let series_text = format!("{{
                {name}
                data: [{data}]
            }},", name = name, data = data);

            series_content += &series_text;

            let min_y_local = s.data.iter().map(|p| {
                let (_, y) = *p;
                y
            }).min_by(|a, b| a.partial_cmp(b).unwrap_or(Equal)).unwrap();

            if min_y_local < min_y {
                min_y = min_y_local;
            }
        }

        let unit = match &self.unit {
            &None => "".to_string(),
            &Some(ref unit) => format!(" ({})", unit)
        };

        let graph_type = if self.area { "area" } else { "line" };

        format!("\
<div id=\"{name}\" style=\"min-width: 310px; height: 400px; margin: 0 auto\"></div>
<script>
    Highcharts.chart('{name}', {{
        chart: {{
            type: '{graph_type}',
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
            }},
            title: {{
                text: '{x_unit}'
            }}
        }},
        credits: {{
            enabled: false
        }},
        series: [{series_content}]
    }});
</script>\
", name = self.name, unit = unit, title = self.name_base, graph_type = graph_type, min_y = min_y, x_unit = self.x_unit, series_content = series_content)
    }
}

impl Series {
    pub fn differentiate(&self) -> Series {
        Series {
            name: self.name.clone(),
            data: util::differention(&self.data),
        }
    }
}