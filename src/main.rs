use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{OriginalUri, RawForm},
    http::Uri,
    response::Redirect,
    routing::{get, post},
};
use fantoccini::ClientBuilder;
use std::{borrow::Cow, time::Duration};
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{Instrument, trace, trace_span};
use url::{Host, Url, form_urlencoded};

const CLIENT_ID: &str = "9e5f94bc-e8a4-4e73-b8be-63364c29d753";
const REDIRECT_URI: &str = "https://localhost";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route(
            "/authorize",
            get(
                async |OriginalUri(original_uri): OriginalUri| -> axum::response::Result<Redirect> {
                    let redirect = authorize(&original_uri)
                        .instrument(trace_span!("/authorize", ?original_uri))
                        .await
                        .map_err(|err| err.to_string())?;

                    Ok(Redirect::to(redirect.as_str()))
                },
            ),
        )
        .route(
            "/token",
            post(
                async |RawForm(raw_form): RawForm| -> axum::response::Result<String> {
                    let query = form_urlencoded::parse(raw_form.as_ref())
                        .map(front_params)
                        .collect::<Vec<_>>();

                    let response = exchange_token(&query)
                        .instrument(trace_span!("/token", ?query))
                        .await
                        .map_err(|err| err.to_string())?;

                    Ok(response)
                },
            ),
        );
    let listener = TcpListener::bind("localhost:1312")
        .await
        .context("bind local server")?;

    // never returns
    axum::serve(listener, app).await.expect("never to end");

    Ok(())
}

async fn authorize(original_uri: &Uri) -> Result<Url> {
    let mut upstream_url =
        Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize")
            .expect("valid upstream url");
    upstream_url.set_query(original_uri.query());
    let mut redirect = Url::parse(
        upstream_url
            .query_pairs()
            .find(|(k, _)| k == "redirect_uri")
            .context("request with a redirect_uri")?
            .1
            .as_ref(),
    )
    .context("valid redirect_url")?;

    let upstream_query = upstream_url
        .query_pairs()
        .map(front_params)
        .collect::<Vec<_>>();
    upstream_url.set_query(None);
    upstream_query.into_iter().for_each(|(k, v)| {
        upstream_url
            .query_pairs_mut()
            .append_pair(k.as_ref(), v.as_str());
    });

    let c = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .context("connect to WebDriver")?;

    c.goto(upstream_url.as_str())
        .await
        .context("goto url with puppet")?;

    loop {
        sleep(Duration::from_secs(1)).await;

        let url = c.current_url().await.context("get url from puppet")?;
        match url.host() {
            Some(Host::Domain("localhost")) => break,
            Some(Host::Domain("login.microsoftonline.com")) => {}
            Some(_) | None => return Err(anyhow!("unexpected redirection")),
        };
    }

    let upstream_redirect = c.current_url().await.context("get url from puppet")?;
    redirect.set_query(upstream_redirect.query());

    c.close().await.context("close WebDriver connection")?;

    trace!("redirect to {}", redirect);

    Ok(redirect)
}

fn front_params((k, v): (Cow<'_, str>, Cow<'_, str>)) -> (String, String) {
    let fronted = match k.as_ref() {
        "client_id" => CLIENT_ID.to_string(),
        "redirect_uri" => REDIRECT_URI.to_string(),
        _ => v.into_owned(),
    };

    (k.into_owned(), fronted)
}

async fn exchange_token(query: &Vec<(String, String)>) -> Result<String> {
    let response = reqwest::Client::new()
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(query)
        .send()
        .await
        .context("send upstream")?
        .text()
        .await
        .context("read upstream response")?;

    trace!("response {}", response);

    Ok(response)
}
