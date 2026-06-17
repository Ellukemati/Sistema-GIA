use chrono::{Duration, Local, NaiveDate};
use rusqlite::Connection;

use crate::{
    models::reserva::Reserva,
    repository::{
        reserva_instrumento_repository::ReservaInstrumentoRepository,
        reserva_repository::ReservaRepository,
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
        Self::validar_ejemplares(&ejemplares)?;
        Self::validar_fechas(&fecha_inicio, &fecha_fin)?;

        for ejemplar_id in &ejemplares {
            let disponible = ReservaRepository::ejemplar_disponible(
                conn,
                *ejemplar_id,
                &fecha_inicio,
                &fecha_fin,
            )
            .map_err(|e| e.to_string())?;

            if !disponible {
                return Err(format!(
                    "El ejemplar {} ya está reservado para esas fechas",
                    ejemplar_id
                ));
            }
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

        let reserva_id = ReservaRepository::crear(conn, &reserva).map_err(|e| e.to_string())?;

        for ejemplar_id in ejemplares {
            ReservaInstrumentoRepository::crear(conn, reserva_id, ejemplar_id)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn validar_ejemplares(ejemplares: &[i64]) -> Result<(), String> {
        if ejemplares.is_empty() {
            return Err("Debe seleccionar al menos un ejemplar".to_string());
        }

        Ok(())
    }

    fn validar_fechas(fecha_inicio: &str, fecha_fin: &str) -> Result<(), String> {
        let inicio = NaiveDate::parse_from_str(fecha_inicio, "%Y-%m-%d")
            .map_err(|_| "Fecha de inicio inválida".to_string())?;

        let fin = NaiveDate::parse_from_str(fecha_fin, "%Y-%m-%d")
            .map_err(|_| "Fecha de fin inválida".to_string())?;

        let hoy = Local::now().date_naive();

        let minimo = hoy + Duration::days(5);

        let maximo = hoy + Duration::days(180);

        if inicio < minimo {
            return Err(format!(
                "La reserva debe comenzar al menos {} días después de hoy",
                5
            ));
        }

        if inicio > maximo {
            return Err("No se puede reservar con más de 6 meses de anticipación".to_string());
        }

        if fin <= inicio {
            return Err("La fecha de fin debe ser posterior a la fecha de inicio".to_string());
        }

        Ok(())
    }

    pub fn cancelar_reserva(conn: &Connection, reserva_id: i64) -> Result<(), String> {
        match ReservaRepository::buscar_por_id(conn, reserva_id) {
            Ok(Some(_)) => {}

            Ok(None) => {
                return Err("La reserva no existe".to_string());
            }

            Err(e) => {
                return Err(format!("Error consultando reserva: {}", e));
            }
        }

        ReservaRepository::cancelar(conn, reserva_id).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn obtener_reservas_usuario(
        conn: &Connection,
        id_usuario: i64,
    ) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_por_usuario(conn, id_usuario).map_err(|e| e.to_string())
    }

    pub fn obtener_todas(conn: &Connection) -> Result<Vec<Reserva>, String> {
        ReservaRepository::listar_todas(conn).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn validar_ejemplares_lista_vacia() {
        let resultado = ReservaService::validar_ejemplares(&[]);

        assert!(resultado.is_err());
    }

    #[test]
    fn validar_ejemplares_lista_no_vacia() {
        let resultado = ReservaService::validar_ejemplares(&[1]);

        assert!(resultado.is_ok());
    }

    #[test]
    fn validar_fechas_inicio_muy_cercano() {
        let hoy = Local::now().date_naive();

        let inicio = (hoy + Duration::days(2)).format("%Y-%m-%d").to_string();

        let fin = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();

        let resultado = ReservaService::validar_fechas(&inicio, &fin);

        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_inicio_muy_lejano() {
        let hoy = Local::now().date_naive();

        let inicio = (hoy + Duration::days(200)).format("%Y-%m-%d").to_string();

        let fin = (hoy + Duration::days(210)).format("%Y-%m-%d").to_string();

        let resultado = ReservaService::validar_fechas(&inicio, &fin);

        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_fin_antes_inicio() {
        let hoy = Local::now().date_naive();

        let inicio = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();

        let fin = (hoy + Duration::days(9)).format("%Y-%m-%d").to_string();

        let resultado = ReservaService::validar_fechas(&inicio, &fin);

        assert!(resultado.is_err());
    }

    #[test]
    fn validar_fechas_validas() {
        let hoy = Local::now().date_naive();

        let inicio = (hoy + Duration::days(10)).format("%Y-%m-%d").to_string();

        let fin = (hoy + Duration::days(20)).format("%Y-%m-%d").to_string();

        let resultado = ReservaService::validar_fechas(&inicio, &fin);

        assert!(resultado.is_ok());
    }
}
