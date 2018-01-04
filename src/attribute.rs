#[derive(Debug, PartialEq)]
pub enum Attribute {
    Hide,
}

impl Attribute {
    pub fn from(attribute_text: &str) -> Result<Attribute, ()> {
        if attribute_text.eq("hide") {
            return Result::Ok(Attribute::Hide);
        }

        Result::Err(())
    }
}