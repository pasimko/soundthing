# audio graph thing

Puredata inspired web daw.

## TODO

## Architecture Roadmap

* Probably need a good way to handle things once ownership is passed on to the
  worklet
* it's possible that we can do message passing for everything here? Just throw
  something at output and catch it in `process`?

## Based on wasm-audio-worklet

[View documentation for this example online][dox] or [View compiled example
online][compiled]

[dox]: https://rustwasm.github.io/docs/wasm-bindgen/examples/wasm-audio-worklet.html
[compiled]: https://wasm-bindgen.netlify.app/exbuild/wasm-audio-worklet/

You can build the example locally with:

```
$ python3 run.py
```

and then visiting http://localhost:8080 in a browser should run the example!
