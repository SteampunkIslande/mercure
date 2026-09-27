use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};

pub struct NoQuery;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NoQuery {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Check if the URI contains a query string
        if request.uri().query().is_some() {
            Outcome::Forward(Status::NotFound)
        } else {
            Outcome::Success(NoQuery)
        }
    }
}
