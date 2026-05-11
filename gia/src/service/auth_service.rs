use crate::models::usuario::Usuario;

pub struct AuthService;

impl AuthService {

    pub fn registrar_cuenta(legajo:i32, nombre:String, apellido:String, email:String, tipo:char, password:String,) ->Result<Usuario,String>{
        //validar mail fiuba, si el usuario ya existe, guardar en bd

    }

    pub fn login (email: String, password: String,)->Result<Cuenta,String>{
        //busca el usuario en la bd compara contaseña y devuelve la cuenta 

    }

    pub fn validar_email_fiuba(email: &str) -> bool {
        email.ends_with("@fi.uba.ar")
    }
    
}
