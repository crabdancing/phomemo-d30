# phomemo-d30

Library & utilities for controlling the Phomemo D30 using a reverse engineered protocol.

This library contains components heavily based on code available in the [polskafan phomemo_d30](https://github.com/polskafan/phomemo_d30) repo,
but takes no code directly from said library. That library in turn is based heavily on the work of others,
including [viver](https://github.com/vivier/phomemo-tools) and [theacodes](https://github.com/theacodes/phomemo_m02s).

The gist of it is that there are several magic sequences sent to the appliance by their 'Print Master' Android app. These were sniffed,
and now can be blindly transmitted by a number of scripts and utilities available on Github. This is one such utility.

# Usage


## CLI usage (imperative, for development / patching, assuming all C dependencies already present)

```sh
git clone 'https://github.com/crabdancing/phomemo-d30'
cd phomemo-d30
cargo run --bin d30-cli -- [ARGS]
```

If already installed (e.g., via `cargo install`) for CLI usage, simply call:
```
d30-cli --help
```

## CLI usage (declarative, Nix)

```
nix run github:crabdancing/phomemo-d30
```

This will fetch dependencies, compile, and run the program all in one go.

## Configuration (imperative)

The program will work without any on-disk config, but if you want to use some of the more sophisticated features, here's a brief explanation of the current state of affairs.

There are two config files. On Linux, they are stored under `$XDG_CONFIG_HOME/phomemo-library`. On most systems, that will be `~/.config/phomemo-library` -- so I'll be using that path in this mini-guide to make this more concrete.

Under this directory, it expects two config files. One is for the library itself:

`~/.config/phomemo-library/phomemo-config.toml`


The next file is for the CLI component:

`~/.config/phomemo-library/phomemo-cli-config.toml`

Check [example-config](https://github.com/crabdancing/phomemo-d30/tree/master/example-config) directory for working example files.

## Configuration, declarative (via NixOS & system flake)

Add to your flake inputs:

```nix
inputs = {
  # Somewhere in your `inputs` context, put a `phomemo-d30` input:
  phomemo-d30.url = "github:crabdancing/phomemo-d30";
};
```

Insert module into `sharedModules`:


```nix
{pkgs, ...}: {
  config = {
    home-manager.sharedModules = [
      inputs.phomemo-d30.homeManagerModules.default
    ];
  };
}
```

In home-manager context, you can then configure via:

```nix
programs.phomemo-d30 = {
  enable = true;
  default_device = "kitchen";
  resolution = {
    my_desk = "40:5B:A4:2F:05:46";
    kitchen = "DB:1E:B4:E7:A3:75";
  };
};

```

This can be configured per-user.
