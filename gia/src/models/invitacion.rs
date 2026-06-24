pub struct Invitacion {
    pub email: String,
    pub token: String,
    pub tipo: String, // "A" para Administrador o "P" para Profesor = Docente
    pub expira_en: i64,
}
