use poem_openapi::{ OpenApi, payload::Json, Object};

pub struct TMSApi;

#[derive(Object)]
struct User
{
    id: Option<i32>,
    name: String,
    color: String
}

#[OpenApi]
impl TMSApi {
    
    #[oai(path = "/tms", method = "post")]
    async fn userdisplay(&self, mut user: Json<User>) -> Json<User> {
        if user.0.id < Some(100) {user.0.id = Some(100);} // Why does this work(even with no input id)?
        Json(user.0)
    }
}