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

        if fecha_inicio > fecha_fin {
            return Err(
                "La fecha de inicio debe ser anterior a la fecha de fin"
                    .to_string()
            );
        }

        let reserva = Reserva {
            id: 0,
            id_usuario,
            fecha_inicio,
            fecha_fin,
            estado: "pendiente".to_string(),
            motivo,
            momento_creacion: "".to_string(),
        };

        ReservaRepository::crear(
            conn,
            &reserva,
        )
        .map_err(|e| e.to_string())?;

        let reserva_id = conn.last_insert_rowid();

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