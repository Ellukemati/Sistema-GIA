use crate::models::usuario::Usuario;

pub struct AuthService;

impl AuthService {

    pub fn registrar_cuenta(legajo:i32, nombre:String, apellido:String, email:String, tipo:char, password:String,) ->Result<Usuario,String>{
        
        if !Self::validar_email_fiuba(&email) {
            return Err(
                "El email debe pertenecer a FIUBA".to_string()
            );
        }

        match UsuarioRepository::buscar_por_email(
            conn,
            &email,
        ) {

            Ok(Some(_)) => {
                return Err(
                    "Ya existe un usuario con ese email".to_string()
                );
            }
            Ok(None) => {}
            Err(_) => {
                return Err(
                    "Error consultando usuarios".to_string()
                );
            }
        }

    }

    pub fn login (email: String, password: String,)->Result<Cuenta,String>{
        //busca el usuario en la bd compara contaseña y devuelve la cuenta 

    }

    pub fn validar_email_fiuba(email: &str) -> bool {
        email.ends_with("@fi.uba.ar")
    }
    
    
}
