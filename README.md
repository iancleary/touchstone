# touchstone

Touchstone (S2P) parser for RF Engineering

## References

- [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
- [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  - Local Version of [Touchstone File Format Specification Version 2.1](./docs/touchstone_ver2_1.pdf)

```bash
touchstone files/ntwk3.s2p
```

which outputs:

```bash
============================
In file files/ntwk3.s2p
'files/ntwk3.s2p' is a Relative path with separators (nested).
Plot HTML generated at files/ntwk3.s2p.html
You can open the plot in your browser at:
file:///C:/Users/icleary/Development/_3_OpenSource/touchstone/files/ntwk3.s2p.html
Attempting to open plot in your default browser...
Success! Opening: file:///C:/Users/iancleary/touchstone/files/ntwk3.s2p.html
```

> This works on Windows, MacOS, and Linux file systems!

and created an html file

[![HTML file created for ntwk3.s2p by running `touchstone files/ntwk3.s2p` in the root of this directory](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html.png)](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html)

You can view the HTML file itself here directly: [examples/ntwk3.s2p.html](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html).
