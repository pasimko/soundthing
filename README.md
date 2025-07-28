# audio graph thing

Puredata inspired web daw.

## TODO
* ring buffer to reduce clicking?
    * if smart can remove mutexes
    * Probably introduce some UI lag to reduce chances of a click
    * ie, when user adjusts the frequency the buffer should change like
        4 frames ahead
    * the way:
        * message passing for UI->oscillators
        * ring buffer for oscillators->JSAudioWorklet
* better mutex handling to fix flashing UI?
* read docs to figure out what process should do when mutex fails
    * currently assuming that I just return false, but i need to check

## Architecture Roadmap

* each node should have a ringbuffer
* the audio thread should read from the ringbuffer of the
    leaf nodes
* what's tricky about this is that each node is changing
    after each `process` call and I don't know how to
    read from their ringbuffers without another reference,
    which excludes a writable 
* how is that meant to work? might ask llm.
* maybe message passing everywhere?
    * create a NodeMessage type (enum?)

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
