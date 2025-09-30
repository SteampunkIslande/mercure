use rocket::get;
use rocket::http::{ContentType, Status};

use include_dir::{Dir, include_dir};

static STATIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");

#[get("/favicon.ico")]
pub async fn favico() -> (Status, (ContentType, &'static [u8])) {
    match STATIC_DIR.get_file("favicon.ico") {
        Some(f) => (Status::Accepted, (ContentType::Icon, f.contents())),
        None => (Status::NotFound, (ContentType::Binary, b"")),
    }
}
