Does your local OAuth2 provider only support thunderbird?
Now you can mimick thunderbird client_id with this auth proxy.

This rust proxy needs a WebDriver proxy running locally (on the standard port 4444) to be able to control a web browser to make the OAuth2 token requests in your name (you will need to log in)
 * I used [geckodriver](https://firefox-source-docs.mozilla.org/testing/geckodriver/) for that

# Howto
Install and run the geckodriver WebDriver for firefox (leave it running):
```bash
cargo install geckodriver # If it complains about you .cargo/bin/ not being on PATH, then run it directly, i.e., ~/.cargo/bin/geckodriver
geckodriver --port 4444 --log debug --binary path/to/firefox/firefox-bin
```

Run the proxy from this folder with:
```bash
cargo run Cargo.toml
```

# Configuration

With these two running, on a non-claws-mail client, you can now set the proxy (http://localhost:1312) as your auth server.
it forwards everything to upstream expect for `client_id` and `redirect_uri`.

The proxy offers two routes
 * `/authorize` for getting the long term token
 * `/token` for exchanging the token w/ a temporary one


# Configuration for claws-mail
Unfortunately, the wonderful claws-mail only has some hardcoded oauth2 servers.

Let's patch the release claws-mail:
```bash
# Please clone claws-mail somwhere OUTSIDE this repo
git clone --depth 1 --branch 4.3.1 git://git.claws-mail.org/claws.git
cd claws

# Patch 1
# Now get the OAuth2 patch from claws-mail
curl https://git.claws-mail.org/\?p\=claws.git\;a\=patch\;h\=28b2c38a9b25f611f844202ba785fa4c9588768e\;hp\=41cbf87342cba1333deb76b1fa9443604a88a83a > OAuth2.patch
# copy in ./claws-mail.patch
# Apply it (ignore whitespace errors)
git apply OAuth2.patch

# Patch 2
# copy ./claws-mail.patch from this repository to claws, let's apply it
git apply claws-mail.patch

# Compile as per usual
# apt install libgdk-pixbuf-2.0-dev libgtk-3-dev libetpan-dev bison flex autopoint gettext
./autoconf
make
sudo make install
```

Now to configure claws-mail:
  - Configure → Create new account
  - IMAP account. Your user name is your email, don't put password.
    The IMAP and SMTP servers are the normal ones.
  - In auth for IMAP and SMTP put OAuth2.
  - On the OAuth2 page, select "EPFL" (leave client id and secret blank) and press [open default browser with request]
  - This will make a request to the proxy (which open a localhost:1312), who will then use the geckodriver to open a window where you should log in
  - After logging in a "Unable to connect" flashes up (this is OK). There should be a window waiting saying OAuth was imported successfully into claws-mail. You can close it.
  - If successful, you will find your password filled in, well done!
