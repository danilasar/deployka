use actix_web::{rt, web, App, HttpRequest, HttpResponse, HttpServer};
use std::io::BufReader;
use std::io::BufRead;
use std::process::{Command, Stdio};
use actix_web_httpauth::{extractors::basic::BasicAuth, middleware::HttpAuthentication};
use actix_web::{
    dev::ServiceRequest, error::ErrorUnauthorized, Error as ActixError,
};

async fn do_auth(
    req: ServiceRequest,
    creds: BasicAuth,
) -> Result<ServiceRequest, (ActixError, ServiceRequest)> {
    if creds.user_id() == "zov" && creds.password() == Some("ebat_azow") {
        Ok(req)
    } else {
        Err((ErrorUnauthorized("nope"), req))
    }
}

//#[post("/apt/upgrade")]
async fn upgrade(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, actix_web::Error> {
    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;
    let _ = stream
        .aggregate_continuations()
        .max_continuation_size(2_usize.pow(10)); // 1kb
    let mut stdout = match Command::new("sh")
        .arg("-C")
        .arg("./deploy.sh")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(s) => s,
        Err(e) => {
            let err = format!("{:#?}", e);
            return Ok(HttpResponse::InternalServerError().body(err))
        }
    };
    let stdout = match stdout.stdout.take() {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::InternalServerError().body("can't take stdout"))
        }
    };
    let mut reader = BufReader::new(stdout);
    let mut buffer = vec![0; 128] ;
    rt::spawn(async move {
        loop {
            let bytes_read = match reader.read_until(b'\n', &mut buffer) {
                Ok(br) => br,
                Err(_) => break
            };
            if bytes_read == 0 { // EOF
                break;
            }
            if let Ok(line) = String::from_utf8(buffer.clone()) {
                let _ = session.text(line).await;
            }
        }
        buffer.clear();
    });
    Ok(res)
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(HttpAuthentication::basic(do_auth))
            .route("/apt/upgrade", web::post().to(upgrade))
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
