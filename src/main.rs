use anyhow::{Context, Result, anyhow};
use axum::{
    Form, Router,
    extract::Query,
    http::HeaderValue,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use fantoccini::ClientBuilder;
use reqwest::header;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{Instrument, trace, trace_span};
use url::{Host, Url};

const CLIENT_ID: &str = "9e5f94bc-e8a4-4e73-b8be-63364c29d753";
const REDIRECT_URI: &str = "https://localhost";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    #[derive(serde::Deserialize, Debug)]
    struct AuthorizeQuery {
        redirect_uri: Url,
    }

    #[derive(serde::Deserialize, Debug)]
    struct TokenForm {
        refresh_token: String,
    }

    let app = Router::new()
        .route(
            "/authorize",
            get(
                async |Query(AuthorizeQuery {
                           redirect_uri: mut redirect,
                       }): Query<AuthorizeQuery>| -> axum::response::Result<Redirect> {
                    let code = authorize().await.map_err(|err| err.to_string())?;
                    redirect
                        .query_pairs_mut()
                        .append_pair("code", code.as_str());

                    Ok(Redirect::to(redirect.as_str()))
                }
            ),
        )
        .route(
            "/token",
            post(
                async |Form(TokenForm { refresh_token }): Form<TokenForm>| -> axum::response::Response {
                    let response = fetch_access_token(&refresh_token)
                        .instrument(trace_span!("/token", ?refresh_token))
                        .await
                        .map_err(|err| err.to_string());


                    let mut ret = response.into_response();

                    ret.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );

                    ret
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

async fn authorize() -> Result<String> {
    let mut upstream_url =
        Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize")
            .expect("valid upstream url");
    upstream_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("client_id", CLIENT_ID)
        .append_pair(
            "scope",
            [
                "https://outlook.office.com/IMAP.AccessAsUser.All",
                "https://outlook.office.com/POP.AccessAsUser.All",
                "https://outlook.office.com/SMTP.Send",
                "offline_access",
            ]
            .join(" ")
            .as_str(),
        );

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

    let url = c.current_url().await.context("get url from puppet")?;
    eprintln!("upstream url: {url}");
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .context("find code in redirect")?
        .1
        .into_owned();

    c.close().await.context("close WebDriver connection")?;

    Ok(code)
}

async fn fetch_access_token(refresh_token: impl AsRef<str>) -> Result<String> {
    let response = reqwest::Client::new()
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_ref()),
            // ("grant_type", "authorization_code"),
            // ("code", refresh_token.as_ref()),
            // ("redirect_uri", "https://localhost"),
        ])
        .send()
        .await
        .context("send upstream")?
        .text()
        .await
        .context("read upstream response")?;

    trace!("response {}", response);

    Ok(response)
}
