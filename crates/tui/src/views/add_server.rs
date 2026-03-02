#[derive(Default)]
pub struct AddServerState {
    pub name: String,
    pub url: String,
    pub field: AddServerField,
}

#[derive(Default, PartialEq)]
pub enum AddServerField {
    #[default]
    Name,
    Url,
}
