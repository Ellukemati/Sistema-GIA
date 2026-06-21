use serde::Serialize;

#[derive(Serialize)]
pub struct ReservaView {
    pub id: i64,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub estado: String,
    pub texto_estado: String,
    pub clase_estado: String,
    pub motivo: String,
    pub equipos: Vec<String>,
    pub dias: i64,
    pub creada: String,
}
