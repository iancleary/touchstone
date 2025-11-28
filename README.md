# touchstone

Touchstone (S2P) parser for RF Engineering

## Installation

```bash
cargo install touchstone
```

This installs an executable in your `.cargo/bin` directory (`.cargo/bin/touchstone`).

## Usage

The `touchstone` executable can be run with a file path as an argument or a directory path as an argument.

### Help

```bash
touchstone --help
```

Outputs:

```bash
📡 Touchstone (s2p, etc.) file parser, plotter, and more - https://github.com/iancleary/touchstone

VERSION:
    0.10.2

USAGE:
     touchstone <FILE_PATH>
     touchstone <DIRECTORY_PATH>
     touchstone cascade <FILE_1> <FILE_2> ... [--name <OUTPUT_NAME>]

     FILE_PATH: path to a s2p file
     DIRECTORY_PATH: path to a directory containing s2p files

     The s2p file(s) are parsed and an interactive plot (html file and js/ folder)
     is created next to the source file(s).

OPTIONS:
      -v, --version    Print version information
      -h, --help       Print help information

EXAMPLES:
     # Single file (Relative path)
     touchstone files/measurements.s2p

     # Directory (Plot all files in folder)
     touchstone files/data_folder

     # Cascade two networks
     touchstone cascade ntwk1.s2p ntwk2.s2p

     # Cascade with custom output name
     touchstone cascade ntwk1.s2p ntwk2.s2p --name result.s2p

     # Bare filename
     touchstone measurement.s2p

     # Windows absolute path
     touchstone C:\Users\data\measurements.s2p

     # Windows UNC path (network path)
     touchstone \\server\mount\folder\measurement.s2p

     # Unix absolute path
     touchstone /home/user/measurements.s2p
```

### File Path

```bash
touchstone files/ntwk3.s2p
```

which outputs:

```bash
============================
Single file detected. Plotting.
In file: files/ntwk3.s2p
'files/ntwk3.s2p' is a Relative path with separators (nested).
Plot HTML generated at files/ntwk3.s2p.html
You can open the plot in your browser at:
file:///Users/iancleary/Development/touchstone/files/ntwk3.s2p.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/Development/touchstone/files/ntwk3.s2p.html
```

> This works on Windows, MacOS, and Linux file systems!

* MacOS: `file:///Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Windows: `file:///C:/Users/iancleary/touchstone/files/ntwk3.s2p.html`
* Linux: `file:///home/iancleary/touchstone/files/ntwk3.s2p.html`

This command created an html file that is interactive, and designed to not have any network dependence.

[![HTML file created for ntwk3.s2p by running `touchstone files/ntwk3.s2p` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/ntwk3.s2p.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html)

You can view the HTML source file itself here directly: [examples/ntwk3.s2p.html](https://github.com/iancleary/touchstone/tree/main/examples/ntwk3.s2p.html).

> Note that the crate's execution templates in the data parsed from the touchstone file's network object.  So this example will only match for the [files/ntwk3.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk3.s2p?raw=true) data.

### Directory Path

```bash
touchstone files/
```

which outputs:

```bash
============================
In directory: files/
Directory detected. Plotting all valid network files in directory.
Found network file: "files/ntwk1.s2p"
Found network file: "files/ntwk2.s2p"
Found network file: "files/ntwk3.s2p"
Plot HTML generated at files/combined_plot.html
You can open the plot in your browser at:
file:///Users/iancleary/Development/touchstone/files/combined_plot.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/Development/touchstone/files/combined_plot.html
```


This command created an html file that is interactive, and designed to not have any network dependence.

[![HTML file created for the files directory by running `touchstone files/` in the root of this directory](https://github.com/iancleary/touchstone/blob/main/examples/ntwk3.s2p.html.png?raw=true)](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html)

You can view the HTML source file itself here directly: [examples/combined_plot.html](https://github.com/iancleary/touchstone/tree/main/examples/combined_plot.html).

> Note that the crate's execution templates in the data parsed from the touchstone file's network object.   So this example will only match for the [files/ntwk1.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk1.s2p?raw=true), [files/ntwk2.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk2.s2p?raw=true), and [files/ntwk3.s2p](https://github.com/iancleary/touchstone/blob/main/files/ntwk3.s2p?raw=true) data.


## References

* [Touchstone Wikipedia entry](https://en.wikipedia.org/wiki/Touchstone_file)
* [Touchstone File Format Specification Version 2.1](https://ibis.org/touchstone_ver2.1/touchstone_ver2_1.pdf)
  * Local Version of [Touchstone File Format Specification Version 2.1](https://github.com/iancleary/touchstone/blob/main//docs/touchstone_ver2_1.pdf)
