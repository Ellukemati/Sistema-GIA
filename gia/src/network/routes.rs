use rouille::{Request, Response};
use rouille::router;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn handle(request: &Request, conn: Arc<Mutex<Connection>>) -> Response {
    router!(request,
        (GET) (/) => {
            Response::text("OK")
        },
        _ => Response::empty_404()
    )
}