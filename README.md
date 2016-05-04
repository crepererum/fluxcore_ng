# fluxcore\_ng

A simple, but fast CSV scatter plotter. It is a replacement for the abandoned [fluxcore](https://github.com/crepererum/fluxcore) project.

**WARNING: This is a very early prototype. It may or may not be useful foryou, it may or may not break your system and it may or may not be developed further.**

## Installation and Usage

You are going to need [Rust 1.8 or higher](https://www.rust-lang.org/) + Cargo (the Rust build + package manger). Then you can run:

    cargo install

After that step (which might take a while) you can run fluxcore\_ng using:

    fluxcore_ng path/to/file.csv

Please note that the CSV file should actually be a true CSV (separators are `,`!), must contain a header for each column and only integer or float data or NA values (which are represented by `?`/`NA`/`na`).

