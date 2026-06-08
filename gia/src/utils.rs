use rouille::Request;

/// Extrae el valor de 'session_token' de los headers HTTP manualmente
pub fn extraer_token_sesion(request: &Request) -> Option<String> {
    if let Some(cookie_header) = request.header("Cookie") {
        // Puede haber varias cookies juntas
        for parte in cookie_header.split(';') {
            let parte = parte.trim();

            // Si la parte empieza con la clave que definimos, extraemos el valor
            if let Some(token) = parte.strip_prefix("session_token=") {
                return Some(token.to_string());
            }
        }
    }
    None
}
