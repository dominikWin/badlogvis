use graph::Graph;
use input::Log;
use input::Value;
use util::hash_string;

#[derive(Debug)]
pub struct Folder {
    pub name: String,
    pub table: Vec<Value>,
    pub logs: Vec<Log>,
    pub graphs: Vec<Graph>,
}

impl Folder {
    pub fn gen_folders(graphs: Vec<Graph>, values: Vec<Value>, logs: Vec<Log>) -> Vec<Self> {
        let mut folders: Vec<Folder> = Vec::new();

        'outer_topic: for graph in graphs {
            for folder in &mut folders {
                if folder.name.eq(&graph.name_folder) {
                    folder.graphs.push(graph);
                    continue 'outer_topic;
                }
            }
            folders.push(Folder {
                name: graph.name_folder.clone(),
                table: Vec::new(),
                logs: Vec::new(),
                graphs: vec![graph],
            });
        }

        'outer_value: for value in values {
            for folder in &mut folders {
                if folder.name.eq(&value.name_folder) {
                    folder.table.push(value);
                    continue 'outer_value;
                }
            }
            folders.push(Folder {
                name: value.name_folder.clone(),
                table: vec![value],
                logs: Vec::new(),
                graphs: Vec::new(),
            });
        }

        'outer_log: for log in logs {
            for folder in &mut folders {
                if folder.name.eq(&log.name_folder) {
                    folder.logs.push(log);
                    continue 'outer_log;
                }
            }
            folders.push(Folder {
                name: log.name_folder.clone(),
                logs: vec![log],
                table: Vec::new(),
                graphs: Vec::new(),
            });
        }

        folders.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });

        folders
    }

    pub fn gen_html(&self) -> String {
        let table = gen_table(&self.table);
        let log_table = gen_log_table(&self.logs);
        let mut graph_content = String::new();
        for topic in &self.graphs {
            graph_content += &topic.gen_highchart();
        }

        if self.name.is_empty() {
            return format!(
                "{table}\n{log_table}\n{graphs}",
                table = table,
                graphs = graph_content,
                log_table = log_table
            );
        }

        let collapse_name = hash_string(&self.name);

        format!(
            r##"
  <div class="panel-group">
    <div class="panel panel-default">
      <div class="panel-heading">
        <h4 class="panel-title">
          <a data-toggle="collapse" href="#collapse_{collapse_name}">{name}</a>
        </h4>
      </div>
      <div id="collapse_{collapse_name}" class="panel-collapse collapse">
        <div class="panel-body">
          {table}
          {log_table}
          {graphs}
        </div>
      </div>
    </div>
  </div>"##,
            name = self.name,
            table = table,
            graphs = graph_content,
            log_table = log_table,
            collapse_name = collapse_name
        )
    }
}

fn gen_table(values: &[Value]) -> String {
    if values.is_empty() {
        return "<!-- Empty table omitted -->\n".to_string();
    }
    let mut rows = String::new();
    for value in values.iter() {
        rows += &format!(
            "<tr><td>{name}</td><td>{value}</td></tr>\n",
            name = value.name_base,
            value = value.value
        );
    }
    format!(r#"<table class="table table-striped"><thead><tr><th>Name</th><th>Value</th></tr></thead><tbody>{rows}</tbody></table>"#, rows = rows)
}

fn gen_log_table(logs: &[Log]) -> String {
    let mut output = "<!-- Log Table -->\n".into();

    for log in logs {
        let content: String = log.lines.clone().unwrap().join("\n");

        let collapse_name = hash_string(&log.name);

        output += format!(r##"<div class="panel panel-info">
            <div class="panel-heading"><a data-toggle="collapse" href="#collapse_{collapse_name}">{name}</a></div>
            <div id="collapse_{collapse_name}" class="panel-collapse collapse in">
            <div class="panel-body">
                <pre>{content}</pre>
            </div>
          </div>
          </div>"##, name = log.name_base, content = content, collapse_name = collapse_name).as_ref();
    }

    output
}
