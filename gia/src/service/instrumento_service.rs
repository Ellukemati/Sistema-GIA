use crate::models::instrumento::Instrumento;

pub struct InstrumentoService;



impl InstrumentoService {

    pub fn aumentar_stock(
        instrumento: &mut Instrumento,
    ) -> Result<(), String> {

        instrumento.stock += cantidad;

        // Actualizar DB

        Ok(())
    }

    pub fn reducir_stock(
        instrumento: &mut Instrumento,
    ) -> Result<(), String> {

        if cantidad <= 0 {
            return Err("La cantidad debe ser mayor a cero".to_string());
        }

        instrumento.stock -= cantidad;

        // Actualizar DB

        Ok(())
    }


}
    
