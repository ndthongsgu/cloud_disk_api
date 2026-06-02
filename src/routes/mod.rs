use actix_web::web;
use crate::middlewares::auth::UserClaims;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // User API
    cfg.service(crate::handlers::user_handler::search_users);
    cfg.service(crate::handlers::user_handler::get_user_detail);
    cfg.service(crate::handlers::user_handler::create_user);
    cfg.service(crate::handlers::user_handler::update_user);
    cfg.service(crate::handlers::user_handler::deactivate_user);
    cfg.service(crate::handlers::user_handler::activate_user);
    cfg.service(crate::handlers::user_handler::select_users);

    // Auth API
    cfg.service(crate::handlers::auth_handler::register);
    cfg.service(crate::handlers::auth_handler::login);
    cfg.service(crate::handlers::auth_handler::refresh);

    // Telegram Bot API
    cfg.service(crate::handlers::telegram_bot_handler::get_bots);
    cfg.service(crate::handlers::telegram_bot_handler::get_bot_detail);
    cfg.service(crate::handlers::telegram_bot_handler::create_bot);
    cfg.service(crate::handlers::telegram_bot_handler::update_bot);
    cfg.service(crate::handlers::telegram_bot_handler::deactivate_bot);
    cfg.service(crate::handlers::telegram_bot_handler::activate_bot);

    // Role API
    cfg.service(crate::handlers::role_handler::get_permissions_template);
    cfg.service(crate::handlers::role_handler::get_roles);
    cfg.service(crate::handlers::role_handler::get_role_detail);
    cfg.service(crate::handlers::role_handler::create_role);
    cfg.service(crate::handlers::role_handler::update_role);
    cfg.service(crate::handlers::role_handler::delete_role);

    // Files & Storage API
    cfg.service(crate::handlers::file_handler::get_files);
    cfg.service(crate::handlers::file_handler::get_starred_files);
    cfg.service(crate::handlers::file_handler::get_trashed_files);
    cfg.service(crate::handlers::file_handler::create_folder);
    cfg.service(crate::handlers::file_handler::get_storage_info);
    cfg.service(crate::handlers::file_handler::trash_file);
    cfg.service(crate::handlers::file_handler::restore_file);
    cfg.service(crate::handlers::file_handler::delete_file_permanently);
    cfg.service(crate::handlers::file_handler::upload_file);

    // Endpoint kiểm thử bảo mật (Protected Route)
    cfg.route("/users/me", web::get().to(get_my_profile));

}

/// Handler API kiểm thử bảo mật bằng JWT
async fn get_my_profile(user: UserClaims) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok().json(user)
}
