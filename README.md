# touchstone

Touchstone (S2P) parser for RF Engineering

## Installation

```bash
cargo install touchstone
```

This installs an executable in your `.cargo/bin` directory (`.cargo/bin/touchstone`).

## Usage

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
file:///Users/iancleary/touchstone/files/ntwk3.s2p.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/touchstone/files/ntwk3.s2p.html
```

> This works on Windows, MacOS, and Linux file systems!

* MacOS: `file:///Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Windows: `file:///C:/Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Linux: `file:///home/iancleary/touchstone/files/ntwk3.s2p.html`

This command created an html file that is interactive, and designed to not have any network dependence.

[![HTML file created for ntwk3.s2p by running `touchstone files/ntwk3.s2p` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/ntwk3.s2p.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html)

You can view the HTML source file itself here directly: [examples/ntwk3.s2p.html](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html).

> Note that the crate's execution templates in the data parsed from the touchstone file's network object.  So this example will only match for the [files/ntwk3.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk3.s2p?raw=true) data.

## References

* [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
* [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  * Local Version of [Touchstone File Format Specification Version 2.1](./docs/touchstone_ver2_1.pdf)
