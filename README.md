# audio graph thing

Puredata inspired web daw.

## TODO
* ring buffer to reduce clicking?
    * if smart can remove mutexes
    * Probably introduce some UI lag to reduce chances of a click
    * ie, when user adjusts the frequency the buffer should change like
        4 frames ahead
* better mutex handling to fix flashing UI?
* read docs to figure out what process should do when mutex fails
    * currently assuming that I just return false, but i need to check


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
