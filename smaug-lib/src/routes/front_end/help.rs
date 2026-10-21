use std::path::PathBuf;

use include_dir::{Dir, include_dir};
use rocket::{get, http::ContentType, response::Redirect};

static DOC_DIR: Dir = include_dir!("documentation/book");

// 1. Catches /smaug/documentation -> Redirects to /smaug/documentation/
#[get("/documentation")]
pub fn doc_redirect() -> Redirect {
    Redirect::permanent("/smaug/documentation/index.html")
}

// 2. Catches /smaug/documentation/ and subpaths like /smaug/documentation/style.css
#[get("/documentation/<path..>")]
pub fn serve_doc(path: PathBuf) -> Option<(ContentType, &'static [u8])> {
    let path_str = path.to_str()?;
    let target = if path_str.is_empty() {
        "index.html"
    } else {
        path_str
    };

    let file = DOC_DIR.get_file(target)?;
    let ext = file.path().extension()?.to_str()?;
    let content_type = ContentType::from_extension(ext).unwrap_or(ContentType::Bytes);

    Some((content_type, file.contents()))
}
