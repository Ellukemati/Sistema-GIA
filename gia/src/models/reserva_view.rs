use serde::Serialize;

#[derive(Serialize)]
pub struct EquipoReservaView {
    pub nombre: String,
    pub codigo_qr: Option<String>,
    pub numero_serie: Option<String>,
    pub patrimonio: Option<String>,
    pub id_interno: i64,
}

#[derive(Serialize)]
pub struct ReservaView {
    pub id: i64,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub estado: String,
    pub texto_estado: String,
    pub clase_estado: String,
    pub motivo: String,
    pub equipos: Vec<EquipoReservaView>,
    pub dias: i64,
    pub creada: String,
}
