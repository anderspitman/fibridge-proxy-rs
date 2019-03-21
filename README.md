**NOTE: This is still an early work in progress and not quite ready for
production at the moment.**

The point of this is to allow your browser to "host" files which can be
streamed over HTTP. This requires a proxy server to handle the HTTP requests
and forward them to the browser over websockets.

Why would this be useful? If the user has a very large file (genomic data files
can easily be in the 20GB-200GB range), and you want to make
[ranged requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Range_requests)
to that file (ie only download specific chunks) as though it were hosted on a
normal server, this will allow that. In [iobio](http://iobio.io/) we use this
to allow our backend system to access a user's local files for processing by
backend tools such as samtools.

# Example usage

First start up the proxy server. We'll assume it's publicly available at
example.com. It's currently hard-coded to listen for both HTTP and websocket
connections on port 9001.

```bash
cd fibridge-proxy-rs
cargo run --release
```

Create a hoster object in the browser and host a file (see
[this page](https://github.com/anderspitman/fibridge-host-js) for information
about the `fibridge-host` library):

```javascript
const fibridge = require('fibridge-host');
fibridge.createHoster({ proxyAddress: 'example.com', port: 9001, secure: false }).then((hoster) => {

  const file = new File(["Hi there"], "file.txt", {
    type: "text/plain",
  });

  const path = '/' + file.name;

  hoster.hostFile(path, file);

  // This is necessary because fibridge automatically generates a randomized
  // hoster id which is included in the path.
  const fullPath = hoster.getHostedPath(path);
  const url = `http://example.com:9001${fullPath}`;

  console.log(url);
});
```

That will print out a URL for the hosted file. You can then retrieve the file
using any http client:

```bash
curl example.com:9001/<hoster-uuid>/file.txt
Hi there
```

Ranged requests work too:
```bash
curl -H "Range: bytes=0-2" example.com:9001/file.txt
Hi
```

# Other implementations
There is an equivalent JavaScript (node) implementation of the proxy server
available
[here](https://github.com/anderspitman/fibridge-proxy-js).
