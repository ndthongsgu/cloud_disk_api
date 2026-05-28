use actix_web::web;
use crate::middlewares::auth::UserClaims;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Hello API
    cfg.service(crate::handlers::user_handler::hello);
    cfg.service(crate::handlers::user_handler::get_users);

    // Auth API
    cfg.service(crate::handlers::auth_handler::register);
    cfg.service(crate::handlers::auth_handler::login);

    // Endpoint kiểm thử bảo mật (Protected Route)
    cfg.route("/users/me", web::get().to(get_my_profile));
}

/// Handler API kiểm thử bảo mật bằng JWT
async fn get_my_profile(user: UserClaims) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok().json(user)
}
