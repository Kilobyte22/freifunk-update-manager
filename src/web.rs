use actix_web::{web, App, HttpServer, Responder, HttpRequest, middleware, FromRequest, ResponseError, HttpResponse};
use crate::MainState;
use std::sync::Arc;
use actix_web::dev::{PayloadStream, Payload};
use futures::future;
use std::fmt;
use std::net::IpAddr;
use crate::graph::UpdatePolicy;

async fn update_check(
    state: web::Data<Arc<MainState>>,
    web::Path((site, branch, file)): web::Path<(String, String, String)>,
    ForwardedFor(ip): ForwardedFor
) -> impl Responder {

    let site_state = state.graphs.get(&(site.clone(), "any".to_owned())).
        or_else(|| state.graphs.get(&(site.clone(), branch)));

    if let Some(site_state) = site_state {
        let locked_graph = site_state.graph.read().await;

        let should_update = if site_state.config.enabled {
            if let Some(node_key) = locked_graph.ip_addrs.get(&ip) {
                let node = locked_graph.nodes.get(*node_key).unwrap();
                let pol = locked_graph.update_policy.get(*node_key).unwrap();
                match pol {
                    UpdatePolicy::Ready => {
                        log::info!(
                        "Host {} is not updated, pushing update and marking it as updated",
                        node.node.hostname
                    );
                        let mut p = site_state.persistent.lock().await;
                        p.update_node(&node.node.node_id);
                        site_state.persistent_saver.clone().send(()).await.unwrap();
                        true
                    },
                    UpdatePolicy::Finished => {
                        log::info!("Host {} is already latest version", node.node.hostname);
                        true
                    }
                    UpdatePolicy::Pending => {
                        log::info!("Host {} is not yet ready to update", node.node.hostname);
                        false
                    }
                    UpdatePolicy::Broken => {
                        log::info!("Host {} is marked as broken, trying to update anyways...", node.node.hostname);
                        true
                    }
                }
            } else {
                site_state.config.update_default
            }
        } else {
            log::info!("Site {} disabled, not performing any action", site)
            false
        };

        Ok(
            if should_update && !site_state.config.dry_run {
                let path = format!("{}/{}", site_state.config.on_update, file);
                HttpResponse::TemporaryRedirect()
                    .header("Location", path)
                    .finish()
            } else {
                let path = format!("{}/{}", site_state.config.on_noupdate, file);
                HttpResponse::TemporaryRedirect()
                    .header("Location", path)
                    .finish()
            }
        )
    } else {
        Err(actix_web::error::ErrorNotFound("404 Not Found"))
    }
}

async fn node_dump(
    state: web::Data<Arc<MainState>>
) -> impl Responder {
    let dump = crate::dump::generate(&*state).await;
    web::Json(dump)
}

pub async fn main(state: Arc<MainState>) -> Result<(), failure::Error> {
    let listen = state.listen_addr.clone();
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(state.clone())
            .service(
                web::resource("/{site}/{branch}/sysupgrade/{file}")
                    .route(web::get().to(update_check))
            )
            .service(
                web::resource("/node_dump.json")
                    .route(web::get().to(node_dump))
            )
    })
        .bind(&listen)?
        .run()
        .await?;
    Ok(())
}

struct ForwardedFor(IpAddr);

impl FromRequest for ForwardedFor {
    type Error = actix_web::Error;
    type Future = future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload<PayloadStream>) -> Self::Future {
        future::ready((|| {
            if let Some(hdr) = req.headers().get("X-Forwarded-For") {
                Ok(ForwardedFor(
                    hdr.to_str()
                        .map_err(|_| StringError::new("Header is invalid value"))?
                        .parse::<IpAddr>()
                        .map_err(|_| StringError::new("Header is invalid value"))?
                        .to_owned()
                ))
            } else {
                Err(StringError::new("X-Forwarded-For not set"))?
            }
        })())
    }
}

#[derive(Debug)]
struct StringError {
    message: String
}

impl StringError {
    fn new(msg: &str) -> StringError {
        StringError {
            message: msg.to_owned()
        }
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.message, f)
    }
}

impl ResponseError for StringError {}