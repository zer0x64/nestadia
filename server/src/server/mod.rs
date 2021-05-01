mod nestadia_ws;

use nestadia_ws::{EmulationState, NestadiaWs};

use std::time::Instant;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};

use futures::future::{ok, Either};

use rand::Rng;

use actix_session::{CookieSession, Session};
use actix_web::{
    dev::{Service, ServiceRequest},
    web, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

use nestadia_core::ExecutionMode;

const ROM_LIST: [&str; 3] = ["Alter Ego", "Cheryl The Goddess", "Flappybird"];

#[derive(Debug, Serialize, Deserialize)]
struct Credentials {
    password: String,
}

async fn emulator_start_param(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let rom_name = req.match_info().get("rom_name").unwrap();

    let rom = match rom_name {
        _ if rom_name == ROM_LIST[0] => &include_bytes!("../../default_roms/Alter_Ego.nes")[..],
        _ if rom_name == ROM_LIST[1] => {
            &include_bytes!("../../default_roms/cheril-the-goddess.nes")[..]
        }
        _ if rom_name == ROM_LIST[2] => &include_bytes!("../../default_roms/flappybird.nes")[..],
        _ => return Ok(HttpResponse::NotFound().into()),
    };

    let websocket = NestadiaWs {
        state: EmulationState::Ready {
            rom,
            exec_mode: ExecutionMode::Ring3,
        },
        heartbeat: Instant::now(),
        custom_rom: vec![],
        custom_rom_len: 0,
    };

    ws::start(websocket, &req, stream)
}

async fn custom_emulator(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs {
        state: EmulationState::Waiting {
            exec_mode: ExecutionMode::Ring3,
        },
        heartbeat: Instant::now(),
        custom_rom: vec![],
        custom_rom_len: 0,
    };

    ws::start(websocket, &req, stream)
}

async fn rom_list(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json(ROM_LIST)
}

async fn dev_emulator(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs {
        state: EmulationState::Ready {
            rom: include_bytes!("../../default_roms/1.Branch_Basics.nes"),
            exec_mode: ExecutionMode::Ring0,
        }, // TODO: Specify flag mode and put vulnerable ROM
        heartbeat: Instant::now(),
        custom_rom: vec![],
        custom_rom_len: 0,
    };

    ws::start(websocket, &req, stream)
}

async fn login(data: web::Json<Credentials>, session: Session) -> impl Responder {
    if verify_password(&data.0.password) {
        session.set("isLoggedIn", true).unwrap();
        HttpResponse::Ok()
    } else {
        HttpResponse::Unauthorized()
    }
}

async fn logout(session: Session) -> impl Responder {
    match session.set("isLoggedIn", false) {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => HttpResponse::InternalServerError(),
    }
}

fn verify_password(password: &str) -> bool {
    let argon2 = Argon2::default();
    let hash = PasswordHash::new("$argon2id$v=19$m=4096,t=3,p=1$eQ1zJ3zuoDXrL6/zrhkxEg$56gPf/5+JrnpJ37o6GgGqHAjsB7g0Tzk+c4cz6QXXSI").unwrap(); // nwTdWyK4uXmzU9HkVwEDVhhe3ENCgkfa
    argon2.verify_password(password.as_bytes(), &hash).is_ok()
}

async fn flag(_req: HttpRequest) -> impl Responder {
    #[cfg(not(feature = "true-flags"))]
    let flag = include_str!("../../flags/flag1-debug.txt");

    #[cfg(feature = "true-flags")]
    let flag = include_str!("../../flags/flag1-prod.txt");

    flag
}

async fn debug_build(_req: HttpRequest) -> impl Responder {
    actix_files::NamedFile::open("./nestadia_debug")
}

#[actix_web::main]
pub async fn actix_main(bind_addr: String, port: u16) -> std::io::Result<()> {
    let mut session_key = [0u8; 32];
    rand::thread_rng().fill(&mut session_key);

    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(CookieSession::signed(&session_key))
            .service(
                // We scope /api/dev/ differently to enforce access control
                web::scope("/api/dev")
                    .wrap_fn(|req, srv| {
                        // Extract the session information
                        let (req, pl) = req.into_parts();
                        let session = Session::extract(&req).into_inner().unwrap();

                        // Reconstruct the request
                        let req = match ServiceRequest::from_parts(req, pl) {
                            Ok(s) => s,
                            Err(_) => panic!(),
                        };

                        // Check if the user is logged in
                        match session.get("isLoggedIn") {
                            Ok(Some(true)) => Either::Right(srv.call(req)),
                            _ => Either::Left(ok(req.into_response(HttpResponse::Unauthorized()))),
                        }
                    })
                    .route("/emulator", web::get().to(dev_emulator))
                    .route("/flag", web::get().to(flag))
                    .route("/debug_build", web::get().to(debug_build)),
            )
            .service(
                web::scope("/api")
                    .route("/emulator/custom", web::get().to(custom_emulator))
                    .route("/emulator/{rom_name}", web::get().to(emulator_start_param))
                    .route("/list", web::get().to(rom_list))
                    .route("/login", web::post().to(login))
                    .route("/logout", web::get().to(logout)),
            )
            .service(
                actix_files::Files::new("/", "client_build")
                    .index_file("index.html")
                    .disable_content_disposition(),
            )
    })
    .bind((bind_addr, port))?
    .run()
    .await
}

// Small code to generate the hash
// #[test]
// fn test() {
//     use argon2::{Argon2, password_hash::{SaltString, PasswordHasher}};
//     let argon2 = Argon2::default();
//     let salt = SaltString::generate(&mut rand::thread_rng());
//     let password_hash = argon2.hash_password_simple(b"nwTdWyK4uXmzU9HkVwEDVhhe3ENCgkfa", salt.as_ref()).unwrap().to_string();

//     assert_eq!(password_hash, "")
// }
