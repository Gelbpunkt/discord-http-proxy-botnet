# discord-http-proxy-botnet

`discord-http-proxy-botnet` is a ratelimited HTTP proxy in front of the Discord API, making use
of [twilight]. It is a heavily modified fork of their original project.

It will behave as a normal ratelimited proxy, until you send the `X-Spam` header with any value.
Then it will distribute outgoing requests across a list of tokens from its configuration, making use of a mix of availability caching, round robin and dark magic. **Warning**: Discord may or may not like this.

### Using it

HTTP clients often support proxies, such as Ruby's [`Net::HTTP`]. Read into your
HTTP client to see how to use it.

Using the spam part of the botnet proxy will usually require manual patching in your library.
Alongside that, you should skip all ratelimit handling on your end.

### Running via Docker

Build the dockerfile and then run it:

```sh
$ docker build . -t http-proxy
$ docker run -itd -e DISCORD_TOKEN="my token" -e EXTRA_TOKEN_FILE="/tmp/tokens.txt" -v $(pwd)/tokens.txt:/tmp/tokens.txt -p 3000:80 http-proxy
```

This will set the discord token to `"my token"` and map the bound port to port
3000 on the host machine. The list of spam tokens will be loaded from the `/tmp/tokens.txt` file.

### Running via Cargo

Build the binary:

```sh
$ cargo build --release
$ DISCORD_TOKEN="my token" EXTRA_TOKEN_FILE="/tmp/tokens.txt" PORT=3000 ./target/release/twilight_http_proxy
```

This will set the discord token to `"my token"` and bind to port 3000. The list of spam tokens will be loaded from the `/tmp/tokens.txt` file.

[twilight]: https://github.com/twilight-rs/twilight
[`Net::HTTP`]: https://ruby-doc.org/stdlib-2.4.1/libdoc/net/http/rdoc/Net/HTTP.html#method-c-new
