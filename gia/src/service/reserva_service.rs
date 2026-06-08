use rusqlite::Connection;

use crate::{
    models::reserva::Reserva,
    repository::{
        reserva_repository::ReservaRepository,
        reserva_instrumento_repository::ReservaInstrumentoRepository,
    },
};

pub struct ReservaService;

impl ReservaService {

    pub fn crear_reserva(
        conn: &Connection,
        id_usuario: i64,
        fecha_inicio: String,
        fecha_fin: String,
        motivo: Option<String>,
        ejemplares: Vec<i64>,
    ) -> Result<(), String> {

        Self::validar_ejemplares(
            &ejemplares,
        )?;

        Self::validar_fechas(
            &fecha_inicio,
            &fecha_fin,
        )?;

        let reserva = Reserva {
            id_usuario,
            fecha_inicio,
            fecha_fin,
            estado: "pendiente".to_string(),
            motivo,
        };

        ReservaRepository::crear(
            conn,
            &reserva,
        )
        .map_err(|e| e.to_string())?;

        let reserva_id =
            conn.last_insert_rowid();

        for ejemplar_id in ejemplares {

            ReservaInstrumentoRepository::crear(
                conn,
                reserva_id,
                ejemplar_id,
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn validar_ejemplares(
        ejemplares: &[i64],
    ) -> Result<(), String> {

        if ejemplares.is_empty() {
            return Err(
                "Debe seleccionar al menos un ejemplar"
                    .to_string()
            );
        }

        Ok(())
    }

    fn validar_fechas(
        fecha_inicio: &str,
        fecha_fin: &str,
    ) -> Result<(), String> {

        if fecha_inicio > fecha_fin {
            return Err(
                "La fecha de inicio debe ser anterior a la fecha de fin"
                    .to_string()
            );
        }

        Ok(())
    }

    pub fn cancelar_reserva(
        conn: &Connection,
        reserva_id: i64,
    ) -> Result<(), String> {

        match ReservaRepository::buscar_por_id(
            conn,
            reserva_id,
        ) {
            Ok(Some(_)) => {}

            Ok(None) => {
                return Err(
                    "La reserva no existe"
                        .to_string()
                );
            }

            Err(e) => {
                return Err(
                    format!(
                        "Error consultando reserva: {}",
                        e
                    )
                );
            }
        }

        ReservaRepository::cancelar(
            conn,
            reserva_id,
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn obtener_reservas_usuario(
        conn: &Connection,
        usuario_id: i64,
    ) -> Result<Vec<Reserva>, String> {

        ReservaRepository::listar_por_usuario(
            conn,
            usuario_id,
        )
        .map_err(|e| e.to_string())
    }

    pub fn obtener_todas(
        conn: &Connection,
    ) -> Result<Vec<Reserva>, String> {

        ReservaRepository::listar_todas(conn)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn validar_ejemplares_lista_vacia() {

        let resultado =
            ReservaService::validar_ejemplares(
                &vec![],
            );

        assert!(
            resultado.is_err()
        );
    }

    #[test]
    fn validar_ejemplares_lista_no_vacia() {

        let resultado =
            ReservaService::validar_ejemplares(
                &vec![1],
            );

        assert!(
            resultado.is_ok()
        );
    }

    #[test]
    fn validar_fechas_es_mayor() {

        let resultado =
            ReservaService::validar_fechas(
                "2026-07-10",
                "2026-07-01",
            );

        assert!(
            resultado.is_err()
        );
    }

    #[test]
    fn validar_fechas_acepta_fechas_validas() {

        let resultado =
            ReservaService::validar_fechas(
                "2026-07-01",
                "2026-07-10",
            );

        assert!(
            resultado.is_ok()
        );
    }

    #[test]
    fn validar_fechas_acepta_mismo_dia() {

        let resultado =
            ReservaService::validar_fechas(
                "2026-07-01",
                "2026-07-01",
            );

        assert!(
            resultado.is_ok()
        );
    }
}