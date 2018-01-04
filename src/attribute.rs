#[derive(Debug, PartialEq)]
pub enum Attribute {
    Hide,
    Area,
    Xaxis,
}

impl Attribute {
    pub fn from(attribute_text: &str) -> Result<Attribute, ()> {
        if attribute_text.eq("hide") {
            return Result::Ok(Attribute::Hide);
        }
        if attribute_text.eq("area") {
            return Result::Ok(Attribute::Area);
        }
        if attribute_text.eq("xaxis") {
            return Result::Ok(Attribute::Xaxis);
        }

        Result::Err(())
    }
}