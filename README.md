your local oauth2 provider only supports thunderbird?
now you can mimick it with this auth proxy.

* requires a WebDriver to get the long term token (refresh token) listening on 4444
 * I used [geckodriver](https://firefox-source-docs.mozilla.org/testing/geckodriver/) for that
 * can be uninstall after first exchange

it offers two routes
 * `/authorize` for getting the long term token
 * `/token` for exchanging the token w/ a temporary one

simply set it in as your auth server and voilà.

it forwards everything to upstream expect for `client_id` and `redirect_uri`.
