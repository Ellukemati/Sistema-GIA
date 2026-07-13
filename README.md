# Taller de Programacion, Grupo 8: Bugbusters

## Integrantes

* Franco Lamas – [flamas@fi.uba.ar](mailto:flamas@fi.uba.ar) – 112306
* Gerónimo Fanti – [gfanti@fi.uba.ar](mailto:gfanti@fi.uba.ar) – 109712
* Oliverio Mourier – [omourier@fi.uba.ar](mailto:omourier@fi.uba.ar) – 106758
* Matias Ezequiel Dundic – [mdundic@fi.uba.ar](mailto:mdundic@fi.uba.ar) – 110773

---
# Configuración Inicial

Antes de ejecutar la aplicación, ya sea en un entorno de desarrollo local o mediante Docker, es necesario configurar las variables de entorno. 

Crear un archivo `.env` en la carpeta raíz del proyecto (dentro de gia) con los siguientes parámetros básicos (ajustar según el entorno):

```env
ADDRESS=0.0.0.0:8080
DB_PATH=data/gia.db
MAILTRAP_USER=tu_usuario_aqui
MAILTRAP_PASSWORD=tu_password_aqui
```
*(Nota: El archivo .env está ignorado en Git por seguridad y no debe subirse al repositorio).*

# Despliegue (Producción)
El proyecto cuenta con la infraestructura necesaria para ser desplegado fácilmente utilizando Docker y Docker Compose. Se utiliza una estrategia Multi-stage build para garantizar una imagen final liviana y segura.

### Requisitos previos
* Docker instalado.
* Docker Compose instalado.

### 💡 Nota para usuarios de Linux (Ubuntu/Debian)

Si al ejecutar los comandos de Docker obtenés un error de "Permiso denegado" (`permission denied`) o el sistema te obliga a usar `sudo` en cada paso, te recomendamos agregar tu usuario al grupo de Docker para facilitar el uso del sistema:

   ```bash
   sudo systemctl enable --now docker
   sudo usermod -aG docker $USER
   ```

Nota: Después de ejecutar este último comando, tenés que cerrar sesión en Ubuntu y volver a entrar (o reiniciar) para que los permisos se apliquen.

### Comandos de administración

1. **Construir e iniciar el ambiente por primera vez (o al haber cambios en el código):**
   ```bash
   docker-compose up --build -d
   ```
   *La aplicación quedará disponible en `http://localhost:8080/inicio`.*

2. **Iniciar el sistema (si ya fue construido previamente):**
   ```bash
   docker-compose up -d
   ```

3. **Ver el estado y los eventos del sistema (Logger):**
   El sistema implementa un logger dual. Los eventos se guardan en el archivo físico persistente (`data/gia.log`) y también se emiten por consola. Para visualizarlos en tiempo real:
   ```bash
   docker-compose logs -f
   ```

4. **Detener y destruir los contenedores:**
   ```bash
   docker-compose down
   ```
   *Nota: Gracias a la configuración de volúmenes, la base de datos y los archivos de log alojados en la carpeta `/data` persistirán en el disco local y no se perderán al destruir el contenedor.*

# Desarrollo Local (sin Docker)

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

