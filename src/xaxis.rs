use attribute::Attribute;
use input::Topic;

#[derive(Debug)]
pub struct XAxis {
    pub unit: String,
    pub name: String,
    pub data: Option<Vec<f64>>,
}

impl From<&[Topic]> for XAxis {
    fn from(topics: &[Topic]) -> Self {
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
    }
}
