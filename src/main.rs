// dumbrouter - Intentionally dumb Docker name-based HTTP router
// Copyright (C) 2022 Violet McKinney <opensource@viomck.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use actix_web::dev::ConnectionInfo;
use actix_web::http::Method;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bollard::container::ListContainersOptions;
use bollard::Docker;
use rand::seq::IteratorRandom;
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct AppData {
    docker: Docker,
    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        // i can't find a better way to do this :(
        let supported_methods = [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
        ];

        let unsupported_methods = [Method::CONNECT, Method::TRACE];

        let mut app = App::new().app_data(web::Data::new(AppData {
            docker: Docker::connect_with_socket_defaults().unwrap(),
            http_client: reqwest::Client::new(),
        }));

        for method in supported_methods {
            app = app.route("{path:.*}", web::method(method).to(handler));
        }

        for method in unsupported_methods {
            app = app.route("{_:.*}", web::method(method).to(unsupported_handler));
        }

        app
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

fn die<T: Debug>(reason: T) -> HttpResponse {
    eprintln!("ERROR: {:?}", reason);
    HttpResponse::InternalServerError()
        .body(format!("Internal Server Error (dumbrouter/{VERSION})"))
}

async fn handler(
    conn: ConnectionInfo,
    req: HttpRequest,
    body: Option<web::Bytes>,
    data: web::Data<AppData>,
    path: web::Path<String>,
) -> impl Responder {
    let host = conn.host().to_string();

    // Remove port - dumbrouter is port-agnostic
    let host = host.split(":").collect::<Vec<_>>()[0];
    let host_parts = host.split(".").map(String::from).collect::<Vec<_>>();
    let service = service_from_host_parts(host_parts);

    let dest_host = dest_host_for_service(&data.docker, &service).await;

    if let Err(err) = dest_host {
        return die(err);
    }

    let dest_host = dest_host.unwrap();

    if dest_host.is_none() {
        return HttpResponse::InternalServerError().body(format!(
            "No backend found for service {service}.  (dumbrouter/{VERSION})"
        ));
    }

    let dest_host = dest_host.unwrap();

    let mut header_map = HeaderMap::new();

    // HACK: actix_http::header::map::HeaderMap and reqwest::header::HeaderMap
    // are BOTH actually http::header::map::HeaderMap.  Thanks to re-exports
    // and similar hacks (and quite possibly a lack of Rust knowledge on my
    // part) we can't use them interchangeably.
    for (k, v) in req.headers() {
        header_map.insert(k, v.clone());
    }

    let url = format!("http://{}/{}", dest_host, path.into_inner());

    let mut builder = data
        .http_client
        .request(req.method().clone(), url)
        .headers(header_map);

    if let Some(body) = body {
        builder = builder.body(body);
    }

    let res = builder.send().await;

    if let Err(err) = res {
        return die(err);
    }

    let res = res.unwrap();

    let mut resp_builder = HttpResponse::build(res.status());

    for header in res.headers() {
        resp_builder.append_header(header);
    }

    let body = res.bytes().await;

    if let Err(err) = body {
        return die(err);
    }

    resp_builder.body(body.unwrap())
}

fn service_from_host_parts(parts: Vec<String>) -> String {
    let len = parts.len();
    match len {
        // localhost in localhost
        1 => parts[0].to_string(),
        // _root for example.com
        2 => "_root".to_string(),
        // a.b.c.d in a.b.c.d.example.com
        _ => parts.split_at(len - 2).0.join("."),
    }
}

async fn dest_host_for_service(
    docker: &Docker,
    service: &String,
) -> Result<Option<String>, bollard::errors::Error> {
    let mut filters = HashMap::new();
    filters.insert("status", vec!["running"]);

    Ok(docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await?
        .iter()
        .filter_map(|c| {
            let names = &c.names;
            if names.is_none() {
                return None;
            }

            let names = names.as_ref().unwrap();
            if names.len() != 1 {
                return None;
            }

            let name = names.get(0).unwrap();

            let start_base = format!("/http-{}", service);
            let start_prod = format!("/http-prod-{}", service);

            if name.len() < start_base.len()
                || name.get(..start_base.len()).unwrap().to_string() != start_base
                    && (name.len() < start_prod.len()
                        || name.get(..start_prod.len()).unwrap().to_string() != start_prod)
            {
                return None;
            }

            let ports = &c.ports;
            if ports.is_none() {
                eprintln!("WARN: Container {} is http, but has no port!", name);
                return None;
            }

            let ports = ports.as_ref().unwrap();
            if ports.len() < 1 {
                eprintln!("WARN: Container {} is http, but has no port!", name);
                return None;
            }

            let ports = ports
                .iter()
                .filter(|p| p.ip.is_some() && p.public_port.is_some())
                .collect::<Vec<_>>();

            if ports.len() < 1 {
                eprintln!(
                    "WARN: Container {} needs 1 eligible port, but has {}!",
                    name,
                    ports.len()
                );
                return None;
            }

            let port = ports.get(0).unwrap();

            Some(format!(
                "{}:{}",
                env::var("LOCALHOST_IP").unwrap_or("host.docker.internal".to_string()),
                port.public_port.as_ref().unwrap()
            ))
        })
        .choose(&mut rand::thread_rng()))
}

async fn unsupported_handler() -> impl Responder {
    HttpResponse::NotImplemented().body(format!(
        "This method is not supported.  (dumbrouter/{VERSION})"
    ))
}
