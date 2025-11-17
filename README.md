# audio graph thing

Puredata inspired web daw. I've never used puredata.

## TODO

## Architecture Roadmap

* Probably need a good way to handle things once ownership is passed on to the
  worklet
* it's possible that we can do message passing for everything here? Just throw
  something at output and catch it in `process`?
* Need to arena this out

```
$ wasm-pack build --target web
$ python server.py
```

and then visiting http://localhost:8080 in a browser should run the example!

### file explanation
lib.rs: This creates the "output" node
