use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

/// Registra un evento tanto en la consola (stdout) como en el archivo gia.log
pub fn info(mensaje: &str) {
    let fecha = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let linea_log = format!("[{}] INFO: {}", fecha, mensaje);

    println!("{}", linea_log);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/gia.log")
    {
        let _ = writeln!(file, "{}", linea_log);
    }
}

pub fn error(mensaje: &str) {
    let fecha = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let linea_log = format!("[{}] ERROR: {}", fecha, mensaje);

    eprintln!("{}", linea_log);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/gia.log")
    {
        let _ = writeln!(file, "{}", linea_log);
    }
}
