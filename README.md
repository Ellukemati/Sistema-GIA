# Taller de Programacion, Grupo 8: Bugbusters

## Integrantes

* Franco Lamas – [flamas@fi.uba.ar](mailto:flamas@fi.uba.ar) – 112306
* Gerónimo Fanti – [gfanti@fi.uba.ar](mailto:gfanti@fi.uba.ar) – 109712
* Oliverio Mourier – [omourier@fi.uba.ar](mailto:omourier@fi.uba.ar) – 106758
* Matias Ezequiel Dundic – [mdundic@fi.uba.ar](mailto:mdundic@fi.uba.ar) – 110773

---

# Como usar

A continuación se detallan los pasos para compilar y ejecutar el programa.

## Compilacion

El proyecto está desarrollado en Rust utilizando Cargo como gestor de dependencias y compilación.

Para instalar todas las dependencias ejecutar:

```
chmod +x setup.sh && ./setup.sh
```

Configuraciones de bash:
```
export LIBRARY_PATH=/usr/local/lib:$LIBRARY_PATH
```

```
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

Para compilar el proyecto ejecutar:

```bash
cargo build
```

---

## Como correr

Para ejecutar el servidor localmente:

```bash
cargo run
```

Luego abrir en el navegador:

```text
http://localhost:8080/inicio
```

---

## Como testear

Para ejecutar todos los tests del proyecto:

```bash
cargo test
```

---

# Desarrollo

## Git hooks (pre-commit)

Para configurar el pre-commit (automatizar el formateo y validación del código) hay que ejecutar (una única vez) estos comandos en la raíz del proyecto:

```bash
chmod +x githooks/pre-commit
git config core.hooksPath githooks
```

El hook ejecuta automáticamente:

* `cargo fmt`
* `cargo clippy`
* `cargo test`

antes de permitir un commit.

---

## Herramientas utilizadas

* Rust
* Cargo
* SQLite
* Rusqlite
* Tera Templates
* HTMX
* Rouille

---

## Arquitectura

El proyecto está organizado en capas:

* `handlers/` → manejo de requests y responses HTTP
* `service/` → lógica de negocio
* `repository/` → acceso a base de datos
* `models/` → estructuras de dominio
* `templates/` → vistas HTML con Tera
* `routes/` → definición de endpoints

---

## Funcionalidades principales

* Registro e inicio de sesión
* Gestión de usuarios
* Reserva de instrumental
* Administración de solicitudes
* Gestión de modelos y ejemplares
* Subida de imágenes y avatares
* Perfil de usuario editable

