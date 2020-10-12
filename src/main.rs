#![deny(clippy::all)]
use actix_web::{
    middleware::Logger,
    web::{route, Bytes, Data},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use http::HeaderMap;
use log::{debug, info};
use std::{
    convert::TryFrom,
    env,
    fs::read_to_string,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};
use twilight_http::{client::Client, request::Request as TwilightRequest, routing::Path};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    }
    env_logger::init();

    let host_raw = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let host = IpAddr::from_str(&host_raw).expect("Invalid host");
    let port = env::var("PORT")
        .unwrap_or_else(|_| "80".into())
        .parse()
        .expect("Invalid port");

    let main_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not found");

    let extra_tokens = match env::var("EXTRA_TOKEN_FILE") {
        Ok(file_name) => {
            let contents = read_to_string(file_name)?;
            contents
                .lines()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
        }
        Err(_) => Vec::new(),
    };

    info!("Using token {}", main_token);
    info!("Using extra tokens {:?}", extra_tokens);

    let main_client = Client::new(main_token);
    let extra_clients = extra_tokens.iter().map(Client::new).collect::<Vec<Client>>();

    let address = SocketAddr::from((host, port));

    let client_data = Data::new(main_client);
    let extra_clients_data = Data::new(extra_clients);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(client_data.clone())
            .app_data(extra_clients_data.clone())
            .default_service(route().to(handle_request))
    })
    .bind(address)?
    .system_exit()
    .run()
    .await
}

async fn handle_request(
    request: HttpRequest,
    bytes: Bytes,
    main_client: Data<Client>,
    extra_clients: Data<Vec<Client>>,
) -> Result<HttpResponse, Error> {
    debug!("Incoming request: {:?}", request);

    // Copy the headers and request attributes
    let uri = request.uri().clone();
    let method = request.method().clone();
    let headers = request.headers().clone();
    let mut final_headers = HeaderMap::new();
    for (h, v) in headers.iter() {
        if h != "X-Spam" {
            final_headers.append(h.clone(), v.clone());
        }
    }

    let trimmed_path = if uri.path().starts_with("/api/v8") {
        uri.path().replace("/api/v8", "")
    } else {
        uri.path().to_owned()
    };
    let path = Path::try_from((method.clone(), trimmed_path.as_ref())).unwrap();

    let path_and_query: String = match uri.path_and_query() {
        Some(v) => v.as_str().replace("/api/v8/", "").into(),
        None => {
            debug!("No path in URI: {:?}", uri);

            return Err(HttpResponse::BadRequest().body("No path in URI").into());
        }
    };

    // Select a client to do the request with
    let client = match headers.get("X-Spam") {
        Some(_) => {
            // Always fall back to main client in case of RL
            let mut res = &**main_client;
            for client in extra_clients.iter() {
                if dbg!(client.time_until_available(&path).await).is_none() {
                    res = client;
                    break;
                }
            }
            res
        },
        None => &**main_client,
    };

    final_headers.insert("Authorization", client.token().unwrap().parse().unwrap());

    let body = if bytes.is_empty() {
        None
    } else {
        Some(bytes.to_vec())
    };
    let raw_request = TwilightRequest {
        body,
        form: None,
        headers: Some(final_headers),
        method,
        path,
        path_str: path_and_query.into(),
    };

    let resp = match client.raw(raw_request).await {
        Ok(r) => r,
        Err(_) => {
            return Err(HttpResponse::InternalServerError()
                .body("Request failed")
                .into())
        }
    };

    let status = resp.status();
    let resp_headers = resp.headers().clone();

    let bytes = match resp.bytes().await {
        Ok(r) => r,
        Err(_) => {
            return Err(HttpResponse::InternalServerError()
                .body("Reading body failed")
                .into())
        }
    };

    let mut builder = HttpResponse::build(status);

    for (h, v) in resp_headers.iter() {
        builder.header(h, v.clone());
    }

    let resp = builder.body(bytes);

    debug!("Response: {:?}", resp);

    Ok(resp)
}
