<div align="center">
  <a href="https://github.com/banyancomputer/banyan-cli" target="_blank">
    <img src=".github/logo.png" alt="Banyan Logo" width="100"></img>
  </a>

  <h1 align="center">Banyan CLI</h1>

  <p>
    <a href="https://codecov.io/gh/banyancomputer/banyan-cli">
      <img src="https://codecov.io/gh/banyancomputer/banyan-cli/branch/master/graph/badge.svg?token=LQL6MA4KSI" alt="Code Coverage"/>
    </a>
    <a href="https://github.com/banyancomputer/banyan-cli/actions?query=">
      <img src="https://github.com/banyancomputer/banyan-cli/actions/workflows/tests_and_checks.yml/badge.svg" alt="Build Status">
    </a>
    <a href="https://github.com/banyancomputer/banyan-cli/blob/main/LICENSE-MIT">
      <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License-MIT">
    </a>
    <a href="https://discord.gg/aHaSw9zgwV">
      <img src="https://img.shields.io/static/v1?label=Discord&message=join%20us!&color=mediumslateblue" alt="Discord">
    </a>
  </p>
</div>

<div align="center"><sub>:warning: Work in progress :warning:</sub></div>

##

## Outline

- [Outline](#outline)
- [What is the Banyan CLI?](#what-is-the-banyan-cli)
- [Installation](#installation)
  - [Using `cargo`](#using-cargo)
- [Usage](#usage)
- [Testing the Project](#testing-the-project)
- [Benchmarking the Project](#benchmarking-the-project)
  - [Configuring the benchmarks](#configuring-the-benchmarks)
  - [Running the benchmarks](#running-the-benchmarks)
  - [Profiling the binary](#profiling-the-binary)
- [Contributing](#contributing)
  - [Formatting](#formatting)
  - [Pre-commit Hook](#pre-commit-hook)
  - [Recommended Development Flow](#recommended-development-flow)
  - [Conventional Commits](#conventional-commits)
- [Getting Help](#getting-help)
- [External Resources](#external-resources)
- [License](#license)


## What is the Banyan CLI?
The Banyan CLI library is a tool for interacting with the [Banyan Filesystem](https://github.com/banyancomputer/banyanfs) and [Banyan Platform](https://beta.data.banyan.computer/). It allows you to turn an ordinary folder on disk into a Drive on our platform, or vice versa.

This integration is seamless, allowing you to keep everything in sync without thinking twice about it. 

## Installation

### Using `cargo`

To install our CLI tool using `cargo`, run:
```console
cargo install --path banyan
```

[//]: # (TODO: Add more installation instructions here as we add more ways to install the project.)

## Usage
The Banyan CLI is easy to use. 
Start by creating a local Key.
```console
banyan keys create
```
You will be asked to give your new Key a name, and prompted to select it for use. 
Select the key for use, then login using this simple command.
```console
banyan account login
```
Be sure to add the Key you just created to the web interface so that your CLI will be able to talk to our servers.

To create a new Drive, run:
```console
banyan drives create --path <PATH>
```
Where `<PATH>` is the local directory you want to create a Drive of.

```console
banyan drives prepare <NAME>
``` 
The `prepare` command finds the correct Drive, scans your filesystem for any changes to it, and encrypts the data locally as a means of preparing it for our platform or your own use. 
If you're logged in, this command will automatically sync your changes to the platform.

To decrypt a Drive, run:
```console
banyan drives restore --name <NAME>
```
The `restore` command reconstruct data in the original directory specified, or create a new directory in your home folder if that directory is no longer available.
If you're logged in, this command will automatically sync up changes before restoring. 
If you've made local changes without preparing, though, they might be overwritten.

## Contributing
:balloon: We're thankful for any feedback and help in improving our project!
We have a [contributing guide](./CONTRIBUTING.md) to help you get involved. We
also adhere to our [Code of Conduct](./CODE_OF_CONDUCT.md).

### Formatting
For formatting Rust in particular, please use `cargo +nightly fmt` as it uses
specific nightly features we recommend by default.
We also enforce `cargo clippy`.

## Getting Help
For usage questions, usecases, or issues reach out to us in our [Discord channel](https://discord.gg/aHaSw9zgwV).
We would be happy to try to answer your question or try opening a new issue on Github.
