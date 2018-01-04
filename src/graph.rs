use std::cmp::Ordering::Equal;

#[derive(Debug)]
pub struct Graph {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: Option<String>,
    pub x_unit: String,
    pub data: Vec<(f64, f64)>,
    pub area: bool,
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
        series: [{{
            //name: '{title}',
            data: [{data}]
        }}]
    }});
</script>\
", name = self.name, unit = unit, title = self.name_base, graph_type = graph_type, data = data, min_y = min_y, x_unit = self.x_unit)
    }
}