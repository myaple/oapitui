#[derive(Default)]
pub struct AddServerState {
    pub name: String,
    pub url: String,
    pub client_cert: String,
    pub client_key: String,
    pub ca_cert: String,
    pub field: AddServerField,
}

#[derive(Default, PartialEq)]
pub enum AddServerField {
    #[default]
    Name,
    Url,
    ClientCert,
    ClientKey,
    CaCert,
}
