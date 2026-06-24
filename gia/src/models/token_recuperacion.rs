pub struct TokenRecuperacion {
    pub id_usuario: i64,
    pub token: String,
    pub expira_en: i64, // En Epoch Unix (segundos)
}
