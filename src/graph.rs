use std::cmp::Ordering::Equal;

use attribute::Attribute;
use input::Topic;
use util;

#[derive(Debug)]
pub struct Graph {
    pub name: String,
    pub name_base: String,
    pub name_folder: String,
    pub unit: String,
    pub x_unit: String,
    pub series: Vec<Series>,
    pub virt: bool,
    pub joinable: bool,
    pub area: bool,
    pub zero: bool,
}

#[derive(Debug)]
pub struct Series {
    pub name: String,
    pub data: Vec<(f64, f64)>,
}

#[derive(Debug)]
pub struct XAxis {
    pub unit: String,
    pub name: String,
    pub data: Option<Vec<f64>>,
}

impl Graph {
    pub fn from_default(
        name: String,
        unit: String,
        x_unit: String,
        series: Vec<Series>,
        virt: bool,
    ) -> Graph {
        let (name_folder, name_base) = util::split_name(&name);
        Graph {
            name,
            name_base,
            name_folder,
            unit,
            x_unit,
            series,
            area: false,
            virt,
            zero: false,
            joinable: false,
        }
    }

    pub fn gen_highchart(&self) -> String {
        let mut series_content = String::new();
        let mut min_y = 0f64;
        for s in &self.series {
            let data = s
                .data
                .iter()
                .map(|p| {
                    let (x, y) = *p;
                    format!("[{},{}]", x, y)
                })
                .collect::<Vec<String>>()
                .join(",");

            let series_text = format!(
                "{{
                name: '{name}',
                data: [{data}]
            }},",
                name = s.name,
                data = data
            );

            series_content += &series_text;

            let min_y_local = s
                .data
                .iter()
                .map(|p| {
                    let (_, y) = *p;
                    y
                })
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Equal))
                .unwrap();

            if min_y_local < min_y {
                min_y = min_y_local;
            }
        }

        let unit = format!(" ({})", self.unit);

        let graph_type = if self.area { "area" } else { "line" };

        let min_y_text = if self.zero {
            format!(
                "yAxis: {{
                min: {min_y}
            }},",
                min_y = min_y
            )
        } else {
            "".to_string()
        };

        let (gen_l, gen_r) = if self.virt {
            ("[ ".to_string(), " ]".to_string())
        } else {
            ("".to_string(), "".to_string())
        };

        format!(
            r#"
<div id="{name}" style="min-width: 310px; height: 400px; margin: 0 auto"></div>
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
            text: '{generated_left}{name}{generated_right}'
        }},
        {min_y_text}
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
</script>
"#,
            name = self.name,
            unit = unit,
            title = self.name_base,
            graph_type = graph_type,
            min_y_text = min_y_text,
            x_unit = self.x_unit,
            series_content = series_content,
            generated_left = gen_l,
            generated_right = gen_r
        )
    }

    pub fn gen_graphs(topics: &[Topic]) -> (Vec<Graph>, XAxis) {
        let xaxis: XAxis = {
            let xaxis_index: Option<usize> = {
                let mut out = Option::None;
                for (i, topic) in topics.iter().enumerate() {
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

            if let Some(xaxis_index) = xaxis_index {
                let unit_text = format!(
                    "{} ({})",
                    topics[xaxis_index].name_base, topics[xaxis_index].unit
                );
                XAxis {
                    name: unit_text,
                    unit: topics[xaxis_index].unit.clone(),
                    data: Option::Some(topics[xaxis_index].data.clone()),
                }
            } else {
                XAxis {
                    name: "Index".to_string(),
                    unit: ::UNITLESS.to_string(),
                    data: Option::None,
                }
            }
        };

        let mut graphs: Vec<Graph> = Vec::new();
        // Scope to stop borrow of xaxis by gen_series
        {
            let gen_series = |data: Vec<f64>, name: String| {
                let data = if let Some(ref xaxis_data) = xaxis.data {
                    util::bind_axis(xaxis_data, &data)
                } else {
                    util::fake_x_axis(&data)
                };
                Series { name, data }
            };

            for i in 0..topics.len() {
                let topic: &Topic = &topics[i];

                // Handle direct
                if !topic.attrs.contains(&Attribute::Hide) {
                    let series = gen_series(topic.data.clone(), topic.name_base.clone());

                    let mut graph = Graph::from_default(
                        topic.name.clone(),
                        topic.unit.clone(),
                        xaxis.name.clone(),
                        vec![series],
                        false,
                    );

                    graph.area = topic.attrs.contains(&Attribute::Area);

                    graph.zero = topic.attrs.contains(&Attribute::Zero);

                    graphs.push(graph);
                }

                // Handle delta
                {
                    if topic.attrs.contains(&Attribute::Delta) {
                        let name = format!("{} Delta", topic.name);

                        let (_, name_base) = util::split_name(&name);

                        let unit = topic.unit.clone();

                        let series = gen_series(topic.data.clone(), name_base).delta();

                        let graph =
                            Graph::from_default(name, unit, xaxis.name.clone(), vec![series], true);

                        graphs.push(graph);
                    }
                }

                // Handle derivative
                {
                    if topic.attrs.contains(&Attribute::Differentiate) {
                        let name = format!("{} Derivative", topic.name);

                        let (_, name_base) = util::split_name(&name);

                        let mut unit = format!("{}/{}", topic.unit, xaxis.unit);

                        let series = gen_series(topic.data.clone(), name_base).differentiate();

                        let graph =
                            Graph::from_default(name, unit, xaxis.name.clone(), vec![series], true);

                        graphs.push(graph);
                    }
                }

                // Handle integral
                {
                    if topic.attrs.contains(&Attribute::Integrate) {
                        let name = format!("{} Integral", topic.name);

                        let (_, name_base) = util::split_name(&name);

                        let mut unit = format!("{}*{}", topic.unit, xaxis.unit);

                        let (series, _total_sum) =
                            gen_series(topic.data.clone(), name_base).integrate();

                        let graph =
                            Graph::from_default(name, unit, xaxis.name.clone(), vec![series], true);

                        graphs.push(graph);
                    }
                }
            }

            // Joins need to run after all direct graphs are added so an invalid join can be detected
            for topic in topics {
                // Handle join
                for attr in &topic.attrs {
                    if let Attribute::Join(join_graph_name) = attr.clone() {
                        let graph = {
                            let join_graph = graphs
                                .iter_mut()
                                .filter(|g| g.name.eq(&join_graph_name))
                                .last();
                            if let Some(join_graph) = join_graph {
                                if !join_graph.joinable {
                                    error!(
                                        "Attempting to join to non-joinable graph {}",
                                        join_graph.name
                                    );
                                }

                                let join_graph: &mut Graph = join_graph;

                                if join_graph
                                    .series
                                    .iter()
                                    .filter(|s| s.name.eq(&topic.name_base))
                                    .count()
                                    > 0
                                {
                                    warning!(
                                        "Attempting to join multiple topics with same name: {}",
                                        topic.name_base
                                    );
                                }

                                if join_graph.unit.ne(&topic.unit) {
                                    warning!(
                                        "Attempting to join different units: {} ({}) and {} ({})",
                                        join_graph.name,
                                        join_graph.unit.clone(),
                                        topic.name,
                                        topic.unit.clone()
                                    );
                                }

                                let series =
                                    gen_series(topic.data.clone(), topic.name_base.clone());

                                join_graph.series.push(series);

                                Option::None
                            } else {
                                let name = join_graph_name;
                                let series =
                                    gen_series(topic.data.clone(), topic.name_base.clone());
                                let mut graph = Graph::from_default(
                                    name,
                                    topic.unit.clone(),
                                    xaxis.name.clone(),
                                    vec![series],
                                    true,
                                );
                                graph.joinable = true;

                                Option::Some(graph)
                            }
                        };
                        if let Some(graph) = graph {
                            graphs.push(graph);
                        }
                    }
                }
            }
        }

        (graphs, xaxis)
    }
}

impl Series {
    pub fn differentiate(&self) -> Series {
        Series {
            name: self.name.clone(),
            data: util::differention(&self.data),
        }
    }

    pub fn integrate(&self) -> (Series, f64) {
        let (data, total_area) = util::integration(&self.data);
        (
            Series {
                name: self.name.clone(),
                data,
            },
            total_area,
        )
    }

    pub fn delta(&self) -> Series {
        Series {
            name: self.name.clone(),
            data: util::delta(&self.data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64;
    use test::Bencher;

    #[bench]
    fn bench_gen_highchart_100_000(b: &mut Bencher) {
        let data = {
            let mut data = Vec::with_capacity(100_000);
            for i in 0..100_000 {
                let point = (
                    (i as f64) * f64::consts::PI,
                    (i as f64) * f64::consts::PI * f64::consts::E,
                );
                data.push(point);
            }
            data
        };
        let series = Series {
            name: "Series".to_string(),
            data,
        };
        let graph = Graph::from_default(
            "test".to_string(),
            "unit".to_string(),
            "time".to_string(),
            vec![series],
            false,
        );

        b.iter(|| graph.gen_highchart());
    }
}
