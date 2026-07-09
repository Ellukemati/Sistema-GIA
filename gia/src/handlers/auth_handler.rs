use crate::constants::EXPIRACION_SESION;
use crate::repository::{
    image_repository::ImageRepository, sesion_repository::SesionRepository,
    usuario_repository::UsuarioRepository,
};
use crate::service::{auth_service::AuthService, image_service::procesar_avatar};
use crate::templates;
use crate::utils::{extraer_token_sesion, usuario_actual};

use rouille::{Request, Response};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use tera::Context;
pub struct AuthHandler;

impl AuthHandler {
    pub fn mostrar_formulario_registro() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render("usuario_registro.html", &ctx))
    }

    pub fn mostrar_bienvenida() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render("bienvenida.html", &ctx))
    }

    pub fn mostrar_formulario_login() -> Response {
        let ctx = Context::new();

        templates::response_html(templates::render("usuario_login.html", &ctx))
    }

    pub fn mostrar_formulario_solicitud() -> Response {
        let ctx = Context::new();
        templates::response_html(templates::render(
            "usuario_solicitar_restablecimiento_password.html",
            &ctx,
        ))
    }

    pub fn mostrar_formulario_cambio(request: &Request) -> Response {
        let token = request.get_param("token").unwrap_or_default();

        if token.is_empty() {
            return templates::response_mensaje_error(
                "Enlace inválido",
                "El token de solicitud de cambio de contraseña no se encuentra en la URL.",
            );
        }

        let mut ctx = Context::new();
        ctx.insert("token", &token);
        templates::response_html(templates::render("usuario_restablecer_password.html", &ctx))
    }

    pub fn mostrar_home(request: &Request, conn: &Connection) -> Response {
        let token = match extraer_token_sesion(request) {
            Some(t) => t,
            None => {
                return Response::redirect_302("/ingreso");
            }
        };

        if let Ok(Some(sesion)) = SesionRepository::buscar_por_token(conn, &token)
            && let Ok(Some(usuario)) = UsuarioRepository::buscar_por_id(conn, sesion.id_usuario)
        {
            let mut ctx = Context::new();
            ctx.insert("usuario_actual", &usuario);
            return templates::response_html(templates::render("home.html", &ctx));
        }
        Response::redirect_302("/ingreso")
    }

    /// Registro publico desde la web
    /// Cualquiera puede mandar la solicitud de registro, pero la cuenta es de tipo Docente
    /// y nace sin el aprobado, requiriendo aprobacion de un administrador
    pub fn procesar_registro(request: &Request, conn: &Connection) -> Response {
        let mut data = match rouille::input::multipart::get_multipart_input(request) {
            Ok(d) => d,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Error",
                    "Formulario multipart inválido.",
                );
            }
        };

        let mut nombre = String::new();
        let mut apellido = String::new();
        let mut email = String::new();
        let mut password = String::new();
        let mut legajo_str = String::new();
        let mut avatar_bytes: Option<Vec<u8>> = None;

        while let Some(mut entry) = data.next() {
            let name = entry.headers.name.clone();
            if entry.headers.filename.is_some() {
                if &*name == "avatar" {
                    let mut bytes = Vec::new();
                    if entry.data.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                        avatar_bytes = Some(bytes);
                    }
                }
            } else {
                let mut text = String::new();
                if entry.data.read_to_string(&mut text).is_ok() {
                    match &*name {
                        "nombre" => nombre = text.trim().to_string(),
                        "apellido" => apellido = text.trim().to_string(),
                        "email" => email = text.trim().to_string(),
                        "password" => password = text,
                        "legajo" => legajo_str = text.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }

        let tipo = "P";
        let legajo = match legajo_str.parse::<i32>() {
            Ok(val) => val,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Datos inválidos",
                    "El legajo debe contener únicamente números y ser válido.",
                );
            }
        };

        match AuthService::registrar_cuenta(conn, legajo, nombre, apellido, email, tipo, &password)
        {
            Ok(_) => {
                if let Some(bytes) = avatar_bytes
                    && let Ok((blob, mime)) = procesar_avatar(&bytes)
                {
                    let _ = ImageRepository::guardar_avatar(conn, legajo as i64, &blob, &mime);
                }

                templates::response_mensaje_exito(
                    "Solicitud de registro enviada",
                    "Tu registro ha sido enviado.\n\
                     Ahora un administrador debe habilitar tu cuenta antes de que puedas iniciar sesión.\n\
                     Te vamos a enviar un correo con la respuesta a tu solicitud.",
                )
            }
            Err(e) => templates::response_mensaje_error(
                "No se pudo completar el registro",
                &e.to_string(),
            ),
        }
    }

    /// Registro privado por invitación de un administrador
    /// Viene de un enlace por email. Como fue enviada por un administrador, se aprueba de inmediato
    pub fn procesar_registro_invitacion(request: &Request, conn: &Connection) -> Response {
        let mut data = match rouille::input::multipart::get_multipart_input(request) {
            Ok(d) => d,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Error",
                    "Formulario multipart inválido.",
                );
            }
        };

        let mut token = String::new();
        let mut nombre = String::new();
        let mut apellido = String::new();
        let mut password = String::new();
        let mut legajo_str = String::new();
        let mut avatar_bytes: Option<Vec<u8>> = None;

        while let Some(mut entry) = data.next() {
            let name = entry.headers.name.clone();
            if entry.headers.filename.is_some() {
                if &*name == "avatar" {
                    let mut bytes = Vec::new();
                    if entry.data.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                        avatar_bytes = Some(bytes);
                    }
                }
            } else {
                let mut text = String::new();
                if entry.data.read_to_string(&mut text).is_ok() {
                    match &*name {
                        "token" => token = text.trim().to_string(),
                        "nombre" => nombre = text.trim().to_string(),
                        "apellido" => apellido = text.trim().to_string(),
                        "password" => password = text,
                        "legajo" => legajo_str = text.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }

        if token.is_empty() {
            return templates::response_mensaje_error(
                "Error de sesión",
                "El token de la invitación es inválido o ha expirado.",
            );
        }

        let legajo = match legajo_str.parse::<i32>() {
            Ok(val) => val,
            Err(_) => {
                return templates::response_mensaje_error(
                    "Formato incorrecto",
                    "El legajo debe contener únicamente números y ser válido.",
                );
            }
        };

        match AuthService::registrar_por_invitacion(
            conn, &token, nombre, apellido, legajo, &password,
        ) {
            Ok(_) => {
                if let Some(bytes) = avatar_bytes
                    && let Ok((blob, mime)) = procesar_avatar(&bytes)
                {
                    let _ = ImageRepository::guardar_avatar(conn, legajo as i64, &blob, &mime);
                }

                templates::response_mensaje_exito(
                    "Cuenta registrada",
                    "Su cuenta fue registrada y habilitada, ya puede iniciar sesión.",
                )
            }
            Err(e) => templates::response_mensaje_error("No se pudo completar el registro", &e),
        }
    }

    pub fn procesar_login(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
        let email = datos_parseados.get("email").cloned().unwrap_or_default();
        let password = datos_parseados.get("password").cloned().unwrap_or_default();

        match AuthService::login(conn, &email, &password) {
            Ok((_usuario, token)) => {
                let cookie_str = format!(
                    "session_token={}; HttpOnly; Path=/; Max-Age={}",
                    token, EXPIRACION_SESION
                );
                Response::empty_204()
                    .with_additional_header("Set-Cookie", cookie_str)
                    .with_additional_header("HX-Redirect", "/inicio")
            }

            Err(e) => templates::response_mensaje_error_con_status(
                "No se pudo iniciar sesión",
                &e.to_string(),
                401,
            ),
        }
    }

    pub fn procesar_logout(request: &Request, conn: &Connection) -> Response {
        if let Some(token) = extraer_token_sesion(request) {
            let _ = SesionRepository::eliminar_por_token(conn, &token);
        }

        Response::redirect_302("/ingreso")
            .with_additional_header("Set-Cookie", "session_token=; HttpOnly; Path=/; Max-Age=0")
    }

    pub fn procesar_solicitud_restablecimiento_password(
        request: &Request,
        conn: &Connection,
    ) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
        let email = datos_parseados.get("email").cloned().unwrap_or_default();

        if email.is_empty() {
            return templates::response_mensaje_error(
                "Falta el correo",
                "Por favor ingrese una dirección de email.",
            );
        }

        // Por seguridad siempre devolvemos un mensaje de éxito aunque el email no exista en la base de datos
        match AuthService::solicitar_restablecimiento_password(conn, &email) {
            Ok(_) => templates::response_mensaje_exito(
                "Proceso iniciado",
                "Si el email ingresado corresponde a una cuenta válida, recibirá un correo con el enlace de restablecimiento.",
            ),
            Err(_) => templates::response_mensaje_exito(
                "Proceso iniciado",
                "Si el email ingresado corresponde a una cuenta válida, recibirá un correo con el enlace de restablecimiento.",
            ),
        }
    }
    pub fn mostrar_perfil(request: &Request, conn: &Connection) -> Response {
        let usuario = match usuario_actual(request, conn) {
            Ok(u) => u,
            Err(r) => return r,
        };

        let mut ctx = Context::new();

        ctx.insert("usuario_actual", &usuario);

        templates::response_html(templates::render("perfil.html", &ctx))
    }
    pub fn actualizar_perfil(request: &Request, conn: &Connection) -> Response {
        let usuario = match usuario_actual(request, conn) {
            Ok(u) => u,

            Err(r) => return r,
        };

        let mut data = match rouille::input::multipart::get_multipart_input(request) {
            Ok(d) => d,

            Err(_) => {
                return templates::response_mensaje_error("Error", "Formulario inválido");
            }
        };

        let mut nombre = usuario.nombre.clone();
        let mut apellido = usuario.apellido.clone();
        let mut password = String::new();
        let mut password_repetida = String::new();
        let mut avatar_bytes: Option<Vec<u8>> = None;

        use std::io::Read;

        while let Some(mut entry) = data.next() {
            let headers = entry.headers.clone();
            let name = headers.name.clone();

            if headers.filename.is_some() {
                if &*name == "avatar" {
                    let mut bytes = Vec::new();

                    if entry.data.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                        avatar_bytes = Some(bytes);
                    }
                }
            } else {
                let mut text = String::new();

                if entry.data.read_to_string(&mut text).is_ok() {
                    match &*name {
                        "nombre" => {
                            nombre = text;
                        }

                        "apellido" => {
                            apellido = text;
                        }
                        "password" => {
                            password = text;
                        }

                        "password_repetida" => {
                            password_repetida = text;
                        }

                        _ => {}
                    }
                }
            }
        }
        if !password.is_empty() && password != password_repetida {
            return templates::response_mensaje_error("Error", "Las contraseñas no coinciden");
        }
        if let Err(e) = UsuarioRepository::actualizar_perfil(conn, usuario.id, &nombre, &apellido) {
            return templates::response_mensaje_error("Error actualizando perfil", &e.to_string());
        }
        if !password.is_empty() {
            if password != password_repetida {
                return templates::response_mensaje_error("Error", "Las contraseñas no coinciden");
            }

            if let Err(e) = AuthService::cambiar_password_usuario(conn, usuario.id, &password) {
                return templates::response_mensaje_error("Error", &e);
            }
        }

        if let Some(bytes) = avatar_bytes
            && let Ok((blob, mime)) = procesar_avatar(&bytes)
        {
            let _ = ImageRepository::guardar_avatar(conn, usuario.legajo as i64, &blob, &mime);
        }

        Response::redirect_302("/perfil")
    }

    pub fn procesar_cambio_password(request: &Request, conn: &Connection) -> Response {
        let mut body = String::new();
        if let Some(mut reader) = request.data() {
            let _ = reader.read_to_string(&mut body);
        }

        let datos_parseados = Self::parsear_formulario(&body);
        let token = datos_parseados.get("token").cloned().unwrap_or_default();
        let password = datos_parseados.get("password").cloned().unwrap_or_default();

        if token.is_empty() {
            return templates::response_mensaje_error(
                "Error de validación",
                "El token de sesión ha expirado o es incorrecto.",
            );
        }

        match AuthService::restablecer_password(conn, &token, &password) {
            Ok(_) => templates::response_mensaje_exito(
                "Contraseña cambiada",
                "Su contraseña ha sido actualizada. Ya puede dirigirse al ingreso y usarla.",
            ),
            Err(e) => templates::response_mensaje_error("No se pudo restablecer la contraseña", &e),
        }
    }

    /// Renderiza la vista de configuración final para el usuario invitado
    pub fn mostrar_formulario_registro_invitacion(
        request: &Request,
        conn: &Connection,
    ) -> Response {
        let token = request.get_param("token").unwrap_or_default();

        if token.is_empty() {
            return templates::response_mensaje_error(
                "Enlace de invitación inválido",
                "El token de acceso no se encuentra en la dirección URL.",
            );
        }

        let ahora_segundos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Buscamos si la invitación es válida y no expiró
        match crate::repository::invitacion_repository::InvitacionRepository::buscar_valido(
            conn,
            &token,
            ahora_segundos,
        ) {
            Ok(Some(invitacion)) => {
                let mut ctx = Context::new();
                ctx.insert("token", &token);

                let tipo_visible = if invitacion.tipo == crate::constants::TIPO_ADMIN {
                    "Administrador"
                } else {
                    "Docente"
                };
                ctx.insert("tipo_cuenta", tipo_visible);

                templates::response_html(templates::render(
                    "usuario_registro_invitacion.html",
                    &ctx,
                ))
            }
            _ => templates::response_mensaje_error(
                "Invitación inválida o expirada",
                "El enlace utilizado ya no es válido, fue utilizado previamente o ha superado el tiempo límite de 24 horas.",
            ),
        }
    }

    fn parsear_formulario(cuerpo: &str) -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        for par in cuerpo.split('&') {
            let mut partes = par.split('=');
            if let (Some(clave), Some(valor)) = (partes.next(), partes.next()) {
                let con_espacios = valor.replace("+", " ");

                let mut bytes = Vec::new();
                let mut i = 0;
                let chars: Vec<char> = con_espacios.chars().collect();

                while i < chars.len() {
                    if chars[i] == '%'
                        && i + 2 < chars.len()
                        && let Some(hex_str) = con_espacios.get(i + 1..i + 3)
                        && let Ok(byte) = u8::from_str_radix(hex_str, 16)
                    {
                        bytes.push(byte);
                        i += 3;
                        continue;
                    }
                    bytes.extend_from_slice(chars[i].to_string().as_bytes());
                    i += 1;
                }

                let valor_decodificado = String::from_utf8(bytes).unwrap_or(con_espacios);
                mapa.insert(clave.to_string(), valor_decodificado.trim().to_string());
            }
        }
        mapa
    }

    pub fn obtener_avatar(conn: &Connection, usuario_id: i64) -> Response {
        let usuario = match UsuarioRepository::buscar_por_id(conn, usuario_id) {
            Ok(Some(u)) => u,

            _ => {
                return Response::empty_404();
            }
        };

        match (usuario.avatar_blob, usuario.avatar_mime) {
            (Some(blob), Some(mime)) => Response::from_data(mime, blob),

            _ => Response::redirect_302("../static/img/avatardefault.avif"),
        }
    }
}
